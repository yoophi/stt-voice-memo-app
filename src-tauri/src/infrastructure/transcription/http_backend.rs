//! HTTP adapter for the application-owned transcription backend.
//!
//! The public types in this module are deliberately wire-oriented. Mapping them
//! to `transcription-core` is kept at the composition seam so HTTP, file paths,
//! bearer credentials, and provider response shapes never leak into the core.

use std::{
    collections::{HashMap, HashSet},
    fmt,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use futures_util::{StreamExt, TryStreamExt};
use reqwest::{
    Client, Response, StatusCode, Url,
    header::{
        AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, HeaderMap, HeaderValue, LOCATION, RETRY_AFTER,
    },
    multipart::{Form, Part},
};
use serde::Deserialize;
use tokio::fs::File;
use tokio_util::{io::ReaderStream, sync::CancellationToken};
use transcription_core::{
    BackendOperationRequest as CoreOperationRequest, BackendRequestId, BackendState,
    BackendTranscript, CleanupDisposition, CreateTranscriptionRequest as CoreCreateRequest,
    Failure as CoreFailure, FailureCategory as CoreFailureCategory, TranscriptionPort,
    TranscriptionPortError,
};

use super::private_source_audio::PrivateSourceAudioStore;

const CREATE_PATH: &str = "v1/transcriptions";
const IDEMPOTENCY_REPLAYED: &str = "Idempotency-Replayed";
const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone)]
pub struct AccessToken(String);

impl AccessToken {
    pub fn new(value: impl Into<String>) -> Result<Self, HttpBackendError> {
        let value = value.into();
        if value.trim().is_empty()
            || value
                .chars()
                .any(|character| matches!(character, '\r' | '\n'))
        {
            return Err(HttpBackendError::InvalidRequest("invalid access token"));
        }
        Ok(Self(value))
    }

    fn bearer_value(&self) -> Result<HeaderValue, HttpBackendError> {
        HeaderValue::from_str(&format!("Bearer {}", self.0))
            .map_err(|_| HttpBackendError::InvalidRequest("invalid access token"))
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AccessToken([REDACTED])")
    }
}

#[derive(Clone, Debug)]
pub struct HttpBackendConfig {
    pub base_url: Url,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    /// Loopback HTTP exists only for hermetic adapter tests.
    pub allow_insecure_loopback: bool,
}

impl HttpBackendConfig {
    pub fn production(base_url: Url) -> Self {
        Self {
            base_url,
            connect_timeout: Duration::from_secs(15),
            request_timeout: Duration::from_secs(120),
            allow_insecure_loopback: false,
        }
    }
}

pub struct CreateUpload {
    pub operation_id: String,
    pub source_audio_id: String,
    pub audio_path: PathBuf,
    pub file_name: String,
    pub media_type: String,
    pub byte_length: u64,
    pub sha256: String,
    pub language_hint: Option<String>,
    pub access_token: AccessToken,
    pub progress: ProgressCallback,
}

pub type ProgressCallback = Arc<dyn Fn(UploadProgress) + Send + Sync>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UploadProgress {
    pub sequence: u64,
    pub supplied_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Clone)]
pub struct HttpTranscriptionBackend {
    client: Client,
    base_url: Url,
    request_timeout: Duration,
    cancellations: Arc<Mutex<CancellationRegistry>>,
    next_generation: Arc<AtomicU64>,
}

/// Hexagonal adapter that resolves opaque source IDs inside app-private storage
/// before handing a streamed request to the wire client.
pub struct CoreHttpTranscriptionPort {
    backend: HttpTranscriptionBackend,
    sources: Arc<PrivateSourceAudioStore>,
}

impl CoreHttpTranscriptionPort {
    pub fn new(backend: HttpTranscriptionBackend, sources: Arc<PrivateSourceAudioStore>) -> Self {
        Self { backend, sources }
    }
}

#[async_trait::async_trait]
impl TranscriptionPort for CoreHttpTranscriptionPort {
    fn cancel_local(&self, operation_id: &transcription_core::TranscriptionOperationId) -> bool {
        self.backend.cancel_local(&operation_id.to_string())
    }

    fn prepare_cancel_reconciliation(
        &self,
        operation_id: &transcription_core::TranscriptionOperationId,
    ) {
        self.backend
            .prepare_cancel_reconciliation(&operation_id.to_string());
    }

    async fn create(
        &self,
        request: CoreCreateRequest,
    ) -> Result<transcription_core::BackendOperation, TranscriptionPortError> {
        let path = self
            .sources
            .resolve_path(&request.source.id)
            .map_err(source_failure)?;
        let operation_id = request.operation_id.clone();
        let attempt = request.attempt;
        let progress_sink = request.progress.clone();
        let upload = CreateUpload {
            operation_id: operation_id.to_string(),
            source_audio_id: request.source.id.to_string(),
            audio_path: path,
            file_name: format!(
                "{}.{}",
                request.source.id.as_str(),
                request.source.file_extension
            ),
            media_type: request.source.media_type,
            byte_length: request.source.byte_length,
            sha256: request.source.sha256,
            language_hint: request.options.language_hint,
            access_token: AccessToken::new(request.authorization.expose_to_adapter())
                .map_err(map_core_error)?,
            progress: Arc::new(move |progress| {
                if let Ok(observation) = transcription_core::UploadObservation::new(
                    operation_id.clone(),
                    attempt,
                    progress.sequence,
                    progress.supplied_bytes,
                    progress.total_bytes,
                ) {
                    progress_sink.observe(observation);
                }
            }),
        };
        self.backend
            .create(upload)
            .await
            .and_then(map_operation)
            .map_err(map_core_error)
    }

    async fn get(
        &self,
        request: CoreOperationRequest,
    ) -> Result<transcription_core::BackendOperation, TranscriptionPortError> {
        let token =
            AccessToken::new(request.authorization.expose_to_adapter()).map_err(map_core_error)?;
        self.backend
            .get(request.backend_operation_id.as_str(), &token)
            .await
            .and_then(map_operation)
            .map_err(map_core_error)
    }

    async fn delete(
        &self,
        request: CoreOperationRequest,
    ) -> Result<transcription_core::BackendOperation, TranscriptionPortError> {
        let token =
            AccessToken::new(request.authorization.expose_to_adapter()).map_err(map_core_error)?;
        let backend_id = request.backend_operation_id.clone();
        let source_id = request.source_audio_id.clone();
        match self
            .backend
            .delete(
                &request.operation_id.to_string(),
                backend_id.as_str(),
                &token,
            )
            .await
            .map_err(map_core_error)?
        {
            Some(operation) => map_operation(operation).map_err(map_core_error),
            None => Ok(transcription_core::BackendOperation {
                id: backend_id,
                source_audio_id: source_id,
                state: BackendState::Deleted,
                result: None,
                failure: None,
                cleanup: CleanupDisposition::Completed,
                request_id: None,
                poll_after_ms: None,
            }),
        }
    }
}

#[derive(Clone)]
struct ActiveRequest {
    generation: u64,
    token: CancellationToken,
}

#[derive(Default)]
struct CancellationRegistry {
    active: HashMap<String, ActiveRequest>,
    pending: HashSet<String>,
}

impl HttpTranscriptionBackend {
    pub fn new(config: HttpBackendConfig) -> Result<Self, HttpBackendError> {
        validate_base_url(&config)?;
        let client = Client::builder()
            .tls_backend_rustls()
            .https_only(!config.allow_insecure_loopback)
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .build()
            .map_err(|_| HttpBackendError::Transport(TransportFailure::Configuration))?;
        Ok(Self {
            client,
            base_url: config.base_url,
            request_timeout: config.request_timeout,
            cancellations: Arc::new(Mutex::new(CancellationRegistry::default())),
            next_generation: Arc::new(AtomicU64::new(1)),
        })
    }

    pub async fn create(&self, upload: CreateUpload) -> Result<BackendOperation, HttpBackendError> {
        validate_create(&upload)?;
        let active_request = self.register(&upload.operation_id);
        let operation_id = upload.operation_id.clone();
        let result = self
            .create_inner(upload, active_request.token.clone())
            .await;
        self.unregister(&operation_id, active_request.generation);
        result
    }

    async fn create_inner(
        &self,
        upload: CreateUpload,
        cancellation: CancellationToken,
    ) -> Result<BackendOperation, HttpBackendError> {
        let metadata = tokio::fs::metadata(&upload.audio_path)
            .await
            .map_err(|_| HttpBackendError::InvalidRequest("audio source is unavailable"))?;
        if !metadata.is_file() || metadata.len() != upload.byte_length {
            return Err(HttpBackendError::InvalidRequest("audio source changed"));
        }

        let file = File::open(&upload.audio_path)
            .await
            .map_err(|_| HttpBackendError::InvalidRequest("audio source is unavailable"))?;
        let progress = progress_stream(file, upload.byte_length, upload.progress);
        let part =
            Part::stream_with_length(reqwest::Body::wrap_stream(progress), upload.byte_length)
                .file_name(upload.file_name)
                .mime_str(&upload.media_type)
                .map_err(|_| HttpBackendError::InvalidRequest("invalid audio media type"))?;
        let mut form = Form::new()
            .part("audio", part)
            .text("source_audio_id", upload.source_audio_id);
        if let Some(language_hint) = upload.language_hint {
            form = form.text("language_hint", language_hint);
        }

        let response = cancellation
            .run_until_cancelled(
                self.client
                    .post(self.endpoint(CREATE_PATH)?)
                    .header(AUTHORIZATION, upload.access_token.bearer_value()?)
                    .header("Idempotency-Key", upload.operation_id)
                    .header("X-Audio-SHA256", upload.sha256)
                    .timeout(self.request_timeout)
                    .multipart(form)
                    .send(),
            )
            .await
            .ok_or(HttpBackendError::Cancelled)?
            .map_err(map_reqwest_error)?;
        parse_operation_response(
            response,
            &[StatusCode::OK, StatusCode::ACCEPTED],
            ResponseContract::Create,
        )
        .await
    }

    pub async fn get(
        &self,
        backend_operation_id: &str,
        access_token: &AccessToken,
    ) -> Result<BackendOperation, HttpBackendError> {
        validate_component(backend_operation_id, "backend operation id")?;
        let response = self
            .client
            .get(self.operation_endpoint(backend_operation_id)?)
            .header(AUTHORIZATION, access_token.bearer_value()?)
            .timeout(self.request_timeout)
            .send()
            .await
            .map_err(map_reqwest_error)?;
        parse_operation_response(response, &[StatusCode::OK], ResponseContract::Resource).await
    }

    pub async fn delete(
        &self,
        local_operation_id: &str,
        backend_operation_id: &str,
        access_token: &AccessToken,
    ) -> Result<Option<BackendOperation>, HttpBackendError> {
        validate_component(local_operation_id, "local operation id")?;
        validate_component(backend_operation_id, "backend operation id")?;
        self.cancel_local(local_operation_id);
        let response = self
            .client
            .delete(self.operation_endpoint(backend_operation_id)?)
            .header(AUTHORIZATION, access_token.bearer_value()?)
            .timeout(self.request_timeout)
            .send()
            .await
            .map_err(map_reqwest_error)?;
        if response.status() == StatusCode::NO_CONTENT {
            ensure_request_id(response.headers())?;
            return Ok(None);
        }
        parse_operation_response(
            response,
            &[StatusCode::ACCEPTED],
            ResponseContract::Resource,
        )
        .await
        .map(Some)
    }

    pub fn cancel_local(&self, operation_id: &str) -> bool {
        let mut registry = self
            .cancellations
            .lock()
            .expect("cancellation registry lock poisoned");
        let token = registry
            .active
            .get(operation_id)
            .map(|active| active.token.clone());
        if let Some(token) = token {
            token.cancel();
            true
        } else {
            registry.pending.insert(operation_id.to_owned());
            true
        }
    }

    pub fn prepare_cancel_reconciliation(&self, operation_id: &str) {
        self.cancellations
            .lock()
            .expect("cancellation registry lock poisoned")
            .pending
            .remove(operation_id);
    }

    fn register(&self, operation_id: &str) -> ActiveRequest {
        let token = CancellationToken::new();
        let active_request = ActiveRequest {
            generation: self.next_generation.fetch_add(1, Ordering::Relaxed),
            token: token.clone(),
        };
        let mut registry = self
            .cancellations
            .lock()
            .expect("cancellation registry lock poisoned");
        if registry.pending.remove(operation_id) {
            token.cancel();
        }
        if let Some(previous) = registry
            .active
            .insert(operation_id.to_owned(), active_request.clone())
        {
            previous.token.cancel();
        }
        active_request
    }

    fn unregister(&self, operation_id: &str, generation: u64) {
        let mut registry = self
            .cancellations
            .lock()
            .expect("cancellation registry lock poisoned");
        if registry
            .active
            .get(operation_id)
            .is_some_and(|current| current.generation == generation)
        {
            registry.active.remove(operation_id);
        }
    }

    fn endpoint(&self, path: &str) -> Result<Url, HttpBackendError> {
        self.base_url
            .join(path)
            .map_err(|_| HttpBackendError::InvalidRequest("invalid backend endpoint"))
    }

    fn operation_endpoint(&self, operation_id: &str) -> Result<Url, HttpBackendError> {
        self.endpoint(&format!("{CREATE_PATH}/{operation_id}"))
    }
}

fn progress_stream(
    file: File,
    total_bytes: u64,
    callback: ProgressCallback,
) -> impl futures_util::TryStream<Ok = bytes::Bytes, Error = std::io::Error> + Send + 'static {
    let supplied = Arc::new(AtomicU64::new(0));
    let sequence = Arc::new(AtomicU64::new(0));
    let last_emission = Arc::new(Mutex::new(None::<Instant>));
    ReaderStream::new(file).inspect_ok(move |chunk| {
        let now_supplied = supplied
            .fetch_add(chunk.len() as u64, Ordering::Relaxed)
            .saturating_add(chunk.len() as u64)
            .min(total_bytes);
        let now = Instant::now();
        let mut last = last_emission.lock().expect("progress lock poisoned");
        let should_emit = now_supplied == total_bytes
            || last.is_none_or(|previous| now.duration_since(previous) >= PROGRESS_INTERVAL);
        if should_emit {
            *last = Some(now);
            callback(UploadProgress {
                sequence: sequence.fetch_add(1, Ordering::Relaxed) + 1,
                supplied_bytes: now_supplied,
                total_bytes,
            });
        }
    })
}

async fn parse_operation_response(
    response: Response,
    allowed: &[StatusCode],
    contract: ResponseContract,
) -> Result<BackendOperation, HttpBackendError> {
    let status = response.status();
    let request_id = ensure_request_id(response.headers())?.to_owned();
    let retry_after_seconds = parse_retry_after(response.headers());
    let location = response
        .headers()
        .get(LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let idempotency_replayed = response
        .headers()
        .get(IDEMPOTENCY_REPLAYED)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == "true");
    let no_store = response
        .headers()
        .get(CACHE_CONTROL)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|directive| directive.trim().eq_ignore_ascii_case("no-store"))
        });
    let expected_content_type = if allowed.contains(&status) {
        "application/json"
    } else {
        "application/problem+json"
    };
    let content_type_valid = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media_type| media_type.trim() == expected_content_type)
        });
    if !content_type_valid || !no_store {
        return Err(HttpBackendError::MalformedResponse {
            request_id: Some(request_id),
        });
    }
    let body = read_limited(response).await?;
    if !allowed.contains(&status) {
        return Err(parse_backend_failure(
            status,
            &request_id,
            retry_after_seconds,
            &body,
        ));
    }
    let wire: WireOperation =
        serde_json::from_slice(&body).map_err(|_| HttpBackendError::MalformedResponse {
            request_id: Some(request_id.clone()),
        })?;
    let mut operation =
        BackendOperation::try_from(wire).map_err(|_| HttpBackendError::MalformedResponse {
            request_id: Some(request_id.clone()),
        })?;
    if operation.request_id != request_id {
        return Err(HttpBackendError::MalformedResponse {
            request_id: Some(request_id),
        });
    }
    let expected_location = format!("/v1/transcriptions/{}", operation.id);
    if status == StatusCode::ACCEPTED
        && (retry_after_seconds.is_none()
            || (contract == ResponseContract::Create
                && location.as_deref() != Some(expected_location.as_str())))
    {
        return Err(HttpBackendError::MalformedResponse {
            request_id: Some(request_id),
        });
    }
    if status == StatusCode::OK && contract == ResponseContract::Create && !idempotency_replayed {
        return Err(HttpBackendError::MalformedResponse {
            request_id: Some(request_id),
        });
    }
    if matches!(
        operation.state,
        OperationState::Queued | OperationState::Processing
    ) && retry_after_seconds.is_none()
    {
        return Err(HttpBackendError::MalformedResponse {
            request_id: Some(request_id),
        });
    }
    operation.retry_after_seconds = retry_after_seconds;
    Ok(operation)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ResponseContract {
    Create,
    Resource,
}

async fn read_limited(response: Response) -> Result<Vec<u8>, HttpBackendError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(HttpBackendError::MalformedResponse { request_id: None });
    }
    let mut stream = response.bytes_stream();
    let mut body = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(map_reqwest_error)?;
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(HttpBackendError::MalformedResponse { request_id: None });
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn ensure_request_id(headers: &HeaderMap) -> Result<&str, HttpBackendError> {
    headers
        .get("X-Request-Id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty() && value.len() <= 256)
        .ok_or(HttpBackendError::MalformedResponse { request_id: None })
}

fn parse_retry_after(headers: &HeaderMap) -> Option<u64> {
    headers
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
}

fn parse_backend_failure(
    status: StatusCode,
    header_request_id: &str,
    header_retry_after: Option<u64>,
    body: &[u8],
) -> HttpBackendError {
    let problem = match serde_json::from_slice::<WireProblem>(body) {
        Ok(problem)
            if problem.status == status.as_u16()
                && problem.request_id == header_request_id
                && valid_failure_tuple(&problem.code, &problem.category, problem.retryable) =>
        {
            problem
        }
        _ => {
            return HttpBackendError::MalformedResponse {
                request_id: Some(header_request_id.to_owned()),
            };
        }
    };
    HttpBackendError::Backend(BackendFailure {
        status: problem.status,
        code: problem.code,
        category: problem.category,
        retryable: problem.retryable,
        retry_after_seconds: problem.retry_after_seconds.or(header_retry_after),
        request_id: problem.request_id,
    })
}

fn map_reqwest_error(error: reqwest::Error) -> HttpBackendError {
    let failure = if error.is_timeout() {
        TransportFailure::Timeout
    } else if error.is_connect() {
        TransportFailure::Connect
    } else if error.is_body() || error.is_decode() {
        TransportFailure::Body
    } else {
        TransportFailure::Request
    };
    HttpBackendError::Transport(failure)
}

fn validate_base_url(config: &HttpBackendConfig) -> Result<(), HttpBackendError> {
    let secure = config.base_url.scheme() == "https";
    let loopback = config.allow_insecure_loopback
        && config.base_url.scheme() == "http"
        && config
            .base_url
            .host_str()
            .is_some_and(|host| host == "127.0.0.1" || host == "::1" || host == "localhost");
    if !secure && !loopback {
        return Err(HttpBackendError::InvalidRequest(
            "backend URL must use HTTPS",
        ));
    }
    if config.base_url.cannot_be_a_base()
        || config.base_url.query().is_some()
        || config.base_url.fragment().is_some()
        || config.base_url.path() != "/"
        || !config.base_url.username().is_empty()
        || config.base_url.password().is_some()
    {
        return Err(HttpBackendError::InvalidRequest("invalid backend URL"));
    }
    Ok(())
}

fn validate_create(upload: &CreateUpload) -> Result<(), HttpBackendError> {
    validate_component(&upload.operation_id, "operation id")?;
    if !(20..=128).contains(&upload.operation_id.len())
        || !upload
            .operation_id
            .bytes()
            .all(|byte| (b'!'..=b'~').contains(&byte))
    {
        return Err(HttpBackendError::InvalidRequest("invalid operation id"));
    }
    validate_component(&upload.source_audio_id, "source audio id")?;
    if upload.byte_length == 0 || upload.byte_length > 25_000_000 {
        return Err(HttpBackendError::InvalidRequest("invalid audio size"));
    }
    if upload.sha256.len() != 64
        || !upload
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(HttpBackendError::InvalidRequest("invalid audio checksum"));
    }
    if upload.file_name.is_empty()
        || upload.file_name.len() > 128
        || upload
            .file_name
            .chars()
            .any(|character| matches!(character, '/' | '\\' | '\r' | '\n' | '\0'))
    {
        return Err(HttpBackendError::InvalidRequest("invalid audio filename"));
    }
    if let Some(language) = &upload.language_hint {
        if !(2..=35).contains(&language.len())
            || !language
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(HttpBackendError::InvalidRequest("invalid language hint"));
        }
    }
    Ok(())
}

fn validate_component(value: &str, name: &'static str) -> Result<(), HttpBackendError> {
    if value.trim().is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|character| matches!(character, '/' | '\\' | '\r' | '\n' | '\0'))
    {
        return Err(HttpBackendError::InvalidRequest(name));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Queued,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Deleting,
    Deleted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CleanupState {
    NotScheduled,
    Scheduled,
    InProgress,
    Completed,
    FailedRetrying,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FailureCategory {
    Retryable,
    UserActionable,
    Terminal,
    Uncertain,
}

#[derive(Clone, Eq, PartialEq)]
pub struct BackendOperation {
    pub id: String,
    pub request_id: String,
    pub source_audio_id: String,
    pub state: OperationState,
    pub created_at: String,
    pub updated_at: String,
    pub result: Option<OperationResult>,
    pub failure: Option<OperationFailure>,
    pub cleanup: CleanupStatus,
    pub retry_after_seconds: Option<u64>,
}

impl fmt::Debug for BackendOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BackendOperation")
            .field("id", &self.id)
            .field("request_id", &self.request_id)
            .field("source_audio_id", &self.source_audio_id)
            .field("state", &self.state)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .field("result", &self.result)
            .field("failure", &self.failure)
            .field("cleanup", &self.cleanup)
            .field("retry_after_seconds", &self.retry_after_seconds)
            .finish()
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperationResult {
    pub text: String,
    pub language: Option<String>,
}

impl fmt::Debug for OperationResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperationResult")
            .field("text", &"[REDACTED]")
            .field("language", &self.language)
            .finish()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperationFailure {
    pub code: String,
    pub category: FailureCategory,
    pub retryable: bool,
    pub retry_after_seconds: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CleanupStatus {
    pub state: CleanupState,
    pub content_available: bool,
    pub delete_by: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireLinks {
    #[serde(rename = "self")]
    self_link: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireOperation {
    id: String,
    request_id: String,
    source_audio_id: String,
    state: OperationState,
    created_at: String,
    updated_at: String,
    result: Option<OperationResult>,
    failure: Option<OperationFailure>,
    cleanup: CleanupStatus,
    links: WireLinks,
}

impl TryFrom<WireOperation> for BackendOperation {
    type Error = ();

    fn try_from(wire: WireOperation) -> Result<Self, Self::Error> {
        validate_component(&wire.id, "id").map_err(|_| ())?;
        validate_component(&wire.source_audio_id, "source id").map_err(|_| ())?;
        if wire.request_id.trim().is_empty()
            || wire.created_at.trim().is_empty()
            || wire.updated_at.trim().is_empty()
            || wire.links.self_link != format!("/v1/transcriptions/{}", wire.id)
        {
            return Err(());
        }
        match wire.state {
            OperationState::Completed
                if wire
                    .result
                    .as_ref()
                    .is_none_or(|result| result.text.trim().is_empty()) =>
            {
                return Err(());
            }
            OperationState::Completed if wire.failure.is_some() => return Err(()),
            OperationState::Completed => {}
            OperationState::Failed if wire.failure.is_none() || wire.result.is_some() => {
                return Err(());
            }
            OperationState::Failed => {
                let failure = wire.failure.as_ref().ok_or(())?;
                if !valid_failure_tuple(&failure.code, &failure.category, failure.retryable) {
                    return Err(());
                }
            }
            _ if wire.result.is_some() || wire.failure.is_some() => return Err(()),
            _ => {}
        }
        let cleanup_requires_deadline = matches!(
            wire.cleanup.state,
            CleanupState::Scheduled | CleanupState::InProgress | CleanupState::FailedRetrying
        );
        if cleanup_requires_deadline != wire.cleanup.delete_by.is_some() {
            return Err(());
        }
        let terminal_or_deleting = matches!(
            wire.state,
            OperationState::Completed
                | OperationState::Failed
                | OperationState::Cancelled
                | OperationState::Deleting
                | OperationState::Deleted
        );
        if (terminal_or_deleting
            && wire.cleanup.content_available
            && wire.cleanup.delete_by.is_none())
            || (!wire.cleanup.content_available && wire.cleanup.state != CleanupState::Completed)
        {
            return Err(());
        }
        Ok(Self {
            id: wire.id,
            request_id: wire.request_id,
            source_audio_id: wire.source_audio_id,
            state: wire.state,
            created_at: wire.created_at,
            updated_at: wire.updated_at,
            result: wire.result,
            failure: wire.failure,
            cleanup: wire.cleanup,
            retry_after_seconds: None,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireProblem {
    #[serde(rename = "type")]
    _problem_type: String,
    #[serde(rename = "title")]
    _title: String,
    status: u16,
    #[serde(rename = "detail")]
    _detail: Option<String>,
    #[serde(rename = "instance")]
    _instance: Option<String>,
    code: String,
    category: FailureCategory,
    retryable: bool,
    request_id: String,
    retry_after_seconds: Option<u64>,
}

fn valid_failure_tuple(code: &str, category: &FailureCategory, retryable: bool) -> bool {
    let expected = match code {
        "MALFORMED_REQUEST"
        | "OPERATION_NOT_FOUND"
        | "CONTENT_EXPIRED"
        | "CHECKSUM_MISMATCH"
        | "IDEMPOTENCY_MISMATCH" => (FailureCategory::Terminal, false),
        "AUTHENTICATION_REQUIRED"
        | "FEATURE_NOT_ALLOWED"
        | "AUDIO_TOO_LARGE"
        | "UNSUPPORTED_AUDIO"
        | "AUDIO_DURATION_EXCEEDED"
        | "INVALID_LANGUAGE_HINT"
        | "USAGE_LIMIT_EXCEEDED" => (FailureCategory::UserActionable, false),
        "RATE_LIMITED" | "PROVIDER_UNAVAILABLE" => (FailureCategory::Retryable, true),
        "OPERATION_CONFLICT" | "INTERNAL_ERROR" | "PROCESSING_TIMEOUT" => {
            (FailureCategory::Uncertain, false)
        }
        _ => return false,
    };
    expected.0 == *category && expected.1 == retryable
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendFailure {
    pub status: u16,
    pub code: String,
    pub category: FailureCategory,
    pub retryable: bool,
    pub retry_after_seconds: Option<u64>,
    pub request_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransportFailure {
    Configuration,
    Connect,
    Timeout,
    Request,
    Body,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HttpBackendError {
    InvalidRequest(&'static str),
    Cancelled,
    Transport(TransportFailure),
    Backend(BackendFailure),
    MalformedResponse { request_id: Option<String> },
}

impl fmt::Display for HttpBackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(reason) => {
                write!(formatter, "invalid transcription request: {reason}")
            }
            Self::Cancelled => formatter.write_str("transcription request cancelled"),
            Self::Transport(kind) => write!(formatter, "transcription transport failure: {kind:?}"),
            Self::Backend(failure) => write!(
                formatter,
                "transcription backend failure {} ({})",
                failure.code, failure.request_id
            ),
            Self::MalformedResponse { request_id } => write!(
                formatter,
                "malformed transcription backend response ({})",
                request_id.as_deref().unwrap_or("missing-request-id")
            ),
        }
    }
}

impl std::error::Error for HttpBackendError {}

fn source_failure(_: transcription_core::SourceAudioError) -> TranscriptionPortError {
    port_failure(
        "SOURCE_AUDIO_UNAVAILABLE",
        CoreFailureCategory::UserActionable,
        None,
        None,
    )
}

fn map_core_error(error: HttpBackendError) -> TranscriptionPortError {
    match error {
        HttpBackendError::Backend(failure) => port_failure(
            &failure.code,
            map_failure_category(failure.category),
            failure
                .retry_after_seconds
                .map(|seconds| seconds.saturating_mul(1_000)),
            BackendRequestId::parse(failure.request_id).ok(),
        ),
        HttpBackendError::Cancelled => port_failure(
            "TRANSFER_CANCELLED",
            CoreFailureCategory::Uncertain,
            None,
            None,
        ),
        HttpBackendError::Transport(TransportFailure::Timeout) => port_failure(
            "PROCESSING_TIMEOUT",
            CoreFailureCategory::Uncertain,
            None,
            None,
        ),
        HttpBackendError::Transport(TransportFailure::Connect) => port_failure(
            "BACKEND_UNAVAILABLE",
            CoreFailureCategory::Retryable,
            Some(1_000),
            None,
        ),
        HttpBackendError::Transport(_) => port_failure(
            "TRANSFER_FAILED",
            CoreFailureCategory::Uncertain,
            None,
            None,
        ),
        HttpBackendError::InvalidRequest(_) => port_failure(
            "INVALID_UPLOAD_REQUEST",
            CoreFailureCategory::Terminal,
            None,
            None,
        ),
        HttpBackendError::MalformedResponse { request_id } => port_failure(
            "MALFORMED_BACKEND_RESPONSE",
            CoreFailureCategory::Terminal,
            None,
            request_id.and_then(|value| BackendRequestId::parse(value).ok()),
        ),
    }
}

fn port_failure(
    code: &str,
    category: CoreFailureCategory,
    retry_after_ms: Option<u64>,
    request_id: Option<BackendRequestId>,
) -> TranscriptionPortError {
    let failure = CoreFailure::new(code, category, retry_after_ms)
        .expect("static adapter failure codes are valid");
    TranscriptionPortError {
        failure: request_id.map_or(failure.clone(), |request_id| {
            failure.with_request_id(request_id)
        }),
    }
}

fn map_failure_category(category: FailureCategory) -> CoreFailureCategory {
    match category {
        FailureCategory::Retryable => CoreFailureCategory::Retryable,
        FailureCategory::UserActionable => CoreFailureCategory::UserActionable,
        FailureCategory::Terminal => CoreFailureCategory::Terminal,
        FailureCategory::Uncertain => CoreFailureCategory::Uncertain,
    }
}

fn map_operation(
    operation: BackendOperation,
) -> Result<transcription_core::BackendOperation, HttpBackendError> {
    let request_id = BackendRequestId::parse(operation.request_id.clone()).map_err(|_| {
        HttpBackendError::MalformedResponse {
            request_id: Some(operation.request_id.clone()),
        }
    })?;
    let failure = operation
        .failure
        .map(|failure| {
            let mut mapped = CoreFailure::new(
                failure.code,
                map_failure_category(failure.category),
                failure
                    .retry_after_seconds
                    .or(operation.retry_after_seconds)
                    .map(|seconds| seconds.saturating_mul(1_000)),
            )
            .map_err(|_| HttpBackendError::MalformedResponse {
                request_id: Some(operation.request_id.clone()),
            })?;
            mapped = mapped.with_request_id(request_id.clone());
            Ok(mapped)
        })
        .transpose()?;
    let result = operation
        .result
        .map(|result| BackendTranscript::new(result.text, result.language));
    Ok(transcription_core::BackendOperation {
        id: transcription_core::BackendOperationId::parse(operation.id).map_err(|_| {
            HttpBackendError::MalformedResponse {
                request_id: Some(operation.request_id.clone()),
            }
        })?,
        source_audio_id: transcription_core::SourceAudioId::parse(operation.source_audio_id)
            .map_err(|_| HttpBackendError::MalformedResponse {
                request_id: Some(operation.request_id.clone()),
            })?,
        state: match operation.state {
            OperationState::Queued => BackendState::Queued,
            OperationState::Processing => BackendState::Processing,
            OperationState::Completed => BackendState::Completed,
            OperationState::Failed => BackendState::Failed,
            OperationState::Cancelled => BackendState::Cancelled,
            OperationState::Deleting => BackendState::Deleting,
            OperationState::Deleted => BackendState::Deleted,
        },
        result,
        failure,
        cleanup: map_cleanup(&operation.cleanup).map_err(|_| {
            HttpBackendError::MalformedResponse {
                request_id: Some(operation.request_id.clone()),
            }
        })?,
        request_id: Some(request_id),
        poll_after_ms: operation
            .retry_after_seconds
            .map(|seconds| seconds.saturating_mul(1_000)),
    })
}

fn map_cleanup(cleanup: &CleanupStatus) -> Result<CleanupDisposition, ()> {
    if !cleanup.content_available {
        return Ok(CleanupDisposition::Completed);
    }
    let delete_by_ms = || {
        let value = cleanup.delete_by.as_deref().ok_or(())?;
        let timestamp =
            time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
                .map_err(|_| ())?;
        u64::try_from(timestamp.unix_timestamp_nanos() / 1_000_000).map_err(|_| ())
    };
    match cleanup.state {
        CleanupState::NotScheduled => Ok(CleanupDisposition::NotScheduled),
        CleanupState::Scheduled => Ok(CleanupDisposition::Scheduled {
            delete_by_ms: delete_by_ms()?,
        }),
        CleanupState::InProgress => Ok(CleanupDisposition::InProgress {
            delete_by_ms: delete_by_ms()?,
        }),
        CleanupState::Completed => Err(()),
        CleanupState::FailedRetrying => Ok(CleanupDisposition::FailedRetrying {
            delete_by_ms: delete_by_ms()?,
        }),
    }
}

#[cfg(test)]
mod cleanup_mapping_tests {
    use super::*;

    #[test]
    fn cleanup_preserves_deadline_and_content_availability() {
        let scheduled = CleanupStatus {
            state: CleanupState::Scheduled,
            content_available: true,
            delete_by: Some("2026-08-16T00:00:00Z".into()),
        };
        assert_eq!(
            map_cleanup(&scheduled).unwrap(),
            CleanupDisposition::Scheduled {
                delete_by_ms: 1_786_838_400_000
            }
        );
        let removed = CleanupStatus {
            state: CleanupState::Completed,
            content_available: false,
            delete_by: None,
        };
        assert_eq!(map_cleanup(&removed), Ok(CleanupDisposition::Completed));
    }
}
