#[allow(unused_imports)]
#[path = "../src/infrastructure/mod.rs"]
pub mod infrastructure;

use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use infrastructure::transcription::{
    AccessToken, CreateUpload, FailureCategory, HttpBackendConfig, HttpBackendError,
    HttpTranscriptionBackend, OperationState, UploadProgress,
};
use tempfile::NamedTempFile;
use url::Url;

const AUDIO: &[u8] = b"bounded-audio-fixture";
const OPERATION_ID: &str = "018f47f2-9f17-7f44-a355-7df30cde0001";
const SHA256: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[tokio::test]
async fn create_streams_exact_headers_and_multipart_contract() {
    let response_body = operation_json("queued", None);
    let (url, request) = spawn_server(
        "202 Accepted",
        &[
            ("X-Request-Id", "request-create"),
            ("Retry-After", "2"),
            ("Location", "/v1/transcriptions/backend-operation"),
        ],
        response_body.as_bytes(),
        Duration::ZERO,
    );
    let backend = backend(url, Duration::from_secs(2));
    let first_fixture = fixture();
    let observations = Arc::new(Mutex::new(Vec::<UploadProgress>::new()));
    let observed = observations.clone();

    let operation = backend
        .create(CreateUpload {
            operation_id: OPERATION_ID.to_owned(),
            source_audio_id: "source-fixture".to_owned(),
            audio_path: first_fixture.path().to_owned(),
            file_name: "recording.m4a".to_owned(),
            media_type: "audio/mp4".to_owned(),
            byte_length: AUDIO.len() as u64,
            sha256: SHA256.to_owned(),
            language_hint: Some("ko-KR".to_owned()),
            access_token: AccessToken::new("test-user-token").unwrap(),
            progress: Arc::new(move |progress| observed.lock().unwrap().push(progress)),
        })
        .await
        .unwrap();

    assert_eq!(operation.state, OperationState::Queued);
    assert_eq!(operation.request_id, "request-create");
    let request = String::from_utf8_lossy(&request.join().unwrap()).to_string();
    assert!(request.starts_with("POST /v1/transcriptions HTTP/1.1\r\n"));
    assert!(request.contains("authorization: Bearer test-user-token\r\n"));
    assert!(request.contains(&format!("idempotency-key: {OPERATION_ID}\r\n")));
    assert!(request.contains(&format!("x-audio-sha256: {SHA256}\r\n")));
    assert!(request.contains("name=\"audio\"; filename=\"recording.m4a\""));
    assert!(request.contains("Content-Type: audio/mp4"));
    assert!(request.contains("name=\"source_audio_id\""));
    assert!(request.contains("source-fixture"));
    assert!(request.contains("name=\"language_hint\""));
    assert!(request.contains("ko-KR"));
    assert!(
        request
            .as_bytes()
            .windows(AUDIO.len())
            .any(|window| window == AUDIO)
    );

    let observations = observations.lock().unwrap();
    assert!(!observations.is_empty());
    assert_eq!(
        observations.last().unwrap().supplied_bytes,
        AUDIO.len() as u64
    );
    assert!(observations.windows(2).all(|pair| {
        pair[0].sequence < pair[1].sequence && pair[0].supplied_bytes <= pair[1].supplied_bytes
    }));
}

#[tokio::test]
async fn get_and_delete_use_exact_resource_path() {
    let get_body = operation_json(
        "completed",
        Some(r#", "result":{"text":"fixture transcript","language":"ko"}"#),
    );
    let (get_url, get_request) = spawn_server(
        "200 OK",
        &[("X-Request-Id", "request-get")],
        get_body.as_bytes(),
        Duration::ZERO,
    );
    let token = AccessToken::new("test-user-token").unwrap();
    let operation = backend(get_url, Duration::from_secs(2))
        .get("backend-operation", &token)
        .await
        .unwrap();
    assert_eq!(operation.state, OperationState::Completed);
    assert_eq!(operation.result.unwrap().text, "fixture transcript");
    let get_request = String::from_utf8_lossy(&get_request.join().unwrap()).to_string();
    assert!(get_request.starts_with("GET /v1/transcriptions/backend-operation HTTP/1.1\r\n"));

    let (delete_url, delete_request) = spawn_server(
        "204 No Content",
        &[("X-Request-Id", "request-delete")],
        &[],
        Duration::ZERO,
    );
    let deleted = backend(delete_url, Duration::from_secs(2))
        .delete(OPERATION_ID, "backend-operation", &token)
        .await
        .unwrap();
    assert!(deleted.is_none());
    let delete_request = String::from_utf8_lossy(&delete_request.join().unwrap()).to_string();
    assert!(delete_request.starts_with("DELETE /v1/transcriptions/backend-operation HTTP/1.1\r\n"));

    let deleting_body = operation_json("deleting", None);
    let (delete_url, delete_request) = spawn_server(
        "202 Accepted",
        &[("X-Request-Id", "request-deleting"), ("Retry-After", "2")],
        deleting_body.as_bytes(),
        Duration::ZERO,
    );
    let deleting = backend(delete_url, Duration::from_secs(2))
        .delete(OPERATION_ID, "backend-operation", &token)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(deleting.state, OperationState::Deleting);
    assert_eq!(deleting.retry_after_seconds, Some(2));
    let delete_request = String::from_utf8_lossy(&delete_request.join().unwrap()).to_string();
    assert!(delete_request.starts_with("DELETE /v1/transcriptions/backend-operation HTTP/1.1\r\n"));
}

#[tokio::test]
async fn backend_failure_is_normalized_without_raw_problem_detail() {
    let problem = br#"{
      "type":"https://example.invalid/problems/rate-limited",
      "title":"Rate limited",
      "status":429,
      "detail":"secret provider payload canary",
      "code":"RATE_LIMITED",
      "category":"retryable",
      "retryable":true,
      "request_id":"request-rate",
      "retry_after_seconds":7
    }"#;
    let (url, request) = spawn_server(
        "429 Too Many Requests",
        &[("X-Request-Id", "request-rate"), ("Retry-After", "7")],
        problem,
        Duration::ZERO,
    );
    let error = backend(url, Duration::from_secs(2))
        .get(
            "backend-operation",
            &AccessToken::new("token-canary").unwrap(),
        )
        .await
        .unwrap_err();
    request.join().unwrap();

    let HttpBackendError::Backend(failure) = &error else {
        panic!("expected normalized backend failure: {error:?}");
    };
    assert_eq!(failure.status, 429);
    assert_eq!(failure.code, "RATE_LIMITED");
    assert_eq!(failure.category, FailureCategory::Retryable);
    assert_eq!(failure.retry_after_seconds, Some(7));
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains("secret provider payload canary"));
    assert!(!diagnostic.contains("token-canary"));
}

#[tokio::test]
async fn local_cancellation_drops_an_in_flight_create() {
    let response_body = operation_json("queued", None);
    let (url, request) = spawn_server(
        "202 Accepted",
        &[("X-Request-Id", "request-late")],
        response_body.as_bytes(),
        Duration::from_secs(2),
    );
    let backend = backend(url, Duration::from_secs(5));
    let second_fixture = fixture();
    let create_backend = backend.clone();
    let create = tokio::spawn(async move {
        create_backend
            .create(CreateUpload {
                operation_id: OPERATION_ID.to_owned(),
                source_audio_id: "source-fixture".to_owned(),
                audio_path: second_fixture.path().to_owned(),
                file_name: "recording.m4a".to_owned(),
                media_type: "audio/mp4".to_owned(),
                byte_length: AUDIO.len() as u64,
                sha256: SHA256.to_owned(),
                language_hint: None,
                access_token: AccessToken::new("test-user-token").unwrap(),
                progress: Arc::new(|_| {}),
            })
            .await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(backend.cancel_local(OPERATION_ID));
    assert_eq!(create.await.unwrap(), Err(HttpBackendError::Cancelled));
    request.join().unwrap();
}

#[tokio::test]
async fn cancellation_before_transport_registration_prevents_upload_start() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
    drop(listener);
    let backend = backend(url, Duration::from_secs(2));
    assert!(backend.cancel_local(OPERATION_ID));
    let fixture = fixture();
    let result = backend
        .create(CreateUpload {
            operation_id: OPERATION_ID.to_owned(),
            source_audio_id: "source-fixture".to_owned(),
            audio_path: fixture.path().to_owned(),
            file_name: "recording.m4a".to_owned(),
            media_type: "audio/mp4".to_owned(),
            byte_length: AUDIO.len() as u64,
            sha256: SHA256.to_owned(),
            language_hint: None,
            access_token: AccessToken::new("test-user-token").unwrap(),
            progress: Arc::new(|_| {}),
        })
        .await;
    assert_eq!(result, Err(HttpBackendError::Cancelled));
}

#[tokio::test]
async fn timeout_and_malformed_response_are_content_safe() {
    let response_body = operation_json("queued", None);
    let (url, request) = spawn_server(
        "202 Accepted",
        &[("X-Request-Id", "request-timeout")],
        response_body.as_bytes(),
        Duration::from_millis(250),
    );
    let error = backend(url, Duration::from_millis(25))
        .get(
            "backend-operation",
            &AccessToken::new("test-user-token").unwrap(),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, HttpBackendError::Transport(_)));
    request.join().unwrap();

    let (url, request) = spawn_server(
        "200 OK",
        &[("X-Request-Id", "request-malformed")],
        br#"{"state":"completed","result":{"text":"leak-canary"}}"#,
        Duration::ZERO,
    );
    let error = backend(url, Duration::from_secs(2))
        .get(
            "backend-operation",
            &AccessToken::new("test-user-token").unwrap(),
        )
        .await
        .unwrap_err();
    request.join().unwrap();
    assert!(matches!(error, HttpBackendError::MalformedResponse { .. }));
    assert!(!format!("{error:?} {error}").contains("leak-canary"));
}

#[tokio::test]
async fn accepted_and_content_responses_require_contract_headers() {
    let body = operation_json("queued", None);
    let (url, request) = spawn_server(
        "202 Accepted",
        &[("X-Request-Id", "request-create"), ("Retry-After", "2")],
        body.as_bytes(),
        Duration::ZERO,
    );
    let missing_header_fixture = fixture();
    let error = backend(url, Duration::from_secs(2))
        .create(CreateUpload {
            operation_id: OPERATION_ID.to_owned(),
            source_audio_id: "source-fixture".to_owned(),
            audio_path: missing_header_fixture.path().to_owned(),
            file_name: "recording.m4a".to_owned(),
            media_type: "audio/mp4".to_owned(),
            byte_length: AUDIO.len() as u64,
            sha256: SHA256.to_owned(),
            language_hint: None,
            access_token: AccessToken::new("test-user-token").unwrap(),
            progress: Arc::new(|_| {}),
        })
        .await
        .unwrap_err();
    request.join().unwrap();
    assert!(matches!(error, HttpBackendError::MalformedResponse { .. }));

    let completed = operation_json(
        "completed",
        Some(r#", "result":{"text":"safe","language":null}"#),
    );
    let (url, request) = spawn_server(
        "200 OK",
        &[
            ("X-Request-Id", "request-get"),
            ("Test-Omit-Cache-Control", "true"),
        ],
        completed.as_bytes(),
        Duration::ZERO,
    );
    let error = backend(url, Duration::from_secs(2))
        .get(
            "backend-operation",
            &AccessToken::new("test-user-token").unwrap(),
        )
        .await
        .unwrap_err();
    request.join().unwrap();
    assert!(matches!(error, HttpBackendError::MalformedResponse { .. }));
}

#[tokio::test]
async fn create_replay_and_problem_responses_require_contract_headers() {
    let completed = operation_json(
        "completed",
        Some(r#", "result":{"text":"safe","language":null}"#),
    );
    let (url, request) = spawn_server(
        "200 OK",
        &[("X-Request-Id", "request-get")],
        completed.as_bytes(),
        Duration::ZERO,
    );
    let replay_fixture = fixture();
    let error = backend(url, Duration::from_secs(2))
        .create(CreateUpload {
            operation_id: OPERATION_ID.to_owned(),
            source_audio_id: "source-fixture".to_owned(),
            audio_path: replay_fixture.path().to_owned(),
            file_name: "recording.m4a".to_owned(),
            media_type: "audio/mp4".to_owned(),
            byte_length: AUDIO.len() as u64,
            sha256: SHA256.to_owned(),
            language_hint: None,
            access_token: AccessToken::new("test-user-token").unwrap(),
            progress: Arc::new(|_| {}),
        })
        .await
        .unwrap_err();
    request.join().unwrap();
    assert!(matches!(error, HttpBackendError::MalformedResponse { .. }));

    let (url, request) = spawn_server(
        "200 OK",
        &[
            ("X-Request-Id", "request-get"),
            ("Idempotency-Replayed", "true"),
        ],
        completed.as_bytes(),
        Duration::ZERO,
    );
    let fixture = fixture();
    let replayed = backend(url, Duration::from_secs(2))
        .create(CreateUpload {
            operation_id: OPERATION_ID.to_owned(),
            source_audio_id: "source-fixture".to_owned(),
            audio_path: fixture.path().to_owned(),
            file_name: "recording.m4a".to_owned(),
            media_type: "audio/mp4".to_owned(),
            byte_length: AUDIO.len() as u64,
            sha256: SHA256.to_owned(),
            language_hint: None,
            access_token: AccessToken::new("test-user-token").unwrap(),
            progress: Arc::new(|_| {}),
        })
        .await
        .unwrap();
    request.join().unwrap();
    assert_eq!(replayed.state, OperationState::Completed);

    let problem = br#"{
      "type":"https://example.invalid/problems/rate-limited",
      "title":"Rate limited",
      "status":429,
      "detail":"safe",
      "code":"RATE_LIMITED",
      "category":"retryable",
      "retryable":true,
      "request_id":"request-rate",
      "retry_after_seconds":7
    }"#;
    let (url, request) = spawn_server(
        "429 Too Many Requests",
        &[
            ("X-Request-Id", "request-rate"),
            ("Retry-After", "7"),
            ("Test-Omit-Cache-Control", "true"),
        ],
        problem,
        Duration::ZERO,
    );
    let error = backend(url, Duration::from_secs(2))
        .get(
            "backend-operation",
            &AccessToken::new("test-user-token").unwrap(),
        )
        .await
        .unwrap_err();
    request.join().unwrap();
    assert!(matches!(error, HttpBackendError::MalformedResponse { .. }));
}

fn backend(base_url: Url, request_timeout: Duration) -> HttpTranscriptionBackend {
    HttpTranscriptionBackend::new(HttpBackendConfig {
        base_url,
        connect_timeout: Duration::from_secs(1),
        request_timeout,
        allow_insecure_loopback: true,
    })
    .unwrap()
}

fn fixture() -> NamedTempFile {
    let mut fixture = NamedTempFile::new().unwrap();
    fixture.write_all(AUDIO).unwrap();
    fixture.flush().unwrap();
    fixture
}

fn operation_json(state: &str, state_fields: Option<&str>) -> String {
    let cleanup = if matches!(
        state,
        "completed" | "failed" | "cancelled" | "deleting" | "deleted"
    ) {
        r#"{"state":"scheduled","content_available":true,"delete_by":"2026-08-16T00:00:00Z"}"#
    } else {
        r#"{"state":"not_scheduled","content_available":true}"#
    };
    format!(
        r#"{{
          "id":"backend-operation",
          "request_id":"request-{state}",
          "source_audio_id":"source-fixture",
          "state":"{state}",
          "created_at":"2026-08-15T00:00:00Z",
          "updated_at":"2026-08-15T00:00:01Z"
          {},
          "cleanup":{cleanup},
          "links":{{"self":"/v1/transcriptions/backend-operation"}}
        }}"#,
        state_fields.unwrap_or("")
    )
    .replace("request-queued", "request-create")
    .replace("request-completed", "request-get")
}

fn spawn_server(
    status: &'static str,
    headers: &[(&'static str, &'static str)],
    response_body: &[u8],
    response_delay: Duration,
) -> (Url, thread::JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let headers = headers.to_vec();
    let response_body = response_body.to_vec();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        let header_end = loop {
            let count = stream.read(&mut buffer).unwrap();
            assert!(count > 0, "request ended before headers");
            request.extend_from_slice(&buffer[..count]);
            if let Some(index) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                break index + 4;
            }
        };
        let header_text = String::from_utf8_lossy(&request[..header_end]);
        let content_length = header_text
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .unwrap_or(0);
        while request.len() - header_end < content_length {
            let count = stream.read(&mut buffer).unwrap();
            assert!(count > 0, "request ended before body");
            request.extend_from_slice(&buffer[..count]);
        }

        thread::sleep(response_delay);
        let content_type = if status
            .as_bytes()
            .first()
            .is_some_and(|byte| matches!(byte, b'4' | b'5'))
        {
            "application/problem+json"
        } else {
            "application/json"
        };
        let omit_cache = headers
            .iter()
            .any(|(name, _)| *name == "Test-Omit-Cache-Control");
        let cache_header = if omit_cache {
            ""
        } else {
            "Cache-Control: no-store\r\n"
        };
        let mut response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: {content_type}\r\n{cache_header}Connection: close\r\n",
            response_body.len(),
        );
        for (name, value) in headers {
            if name == "Test-Omit-Cache-Control" {
                continue;
            }
            response.push_str(&format!("{name}: {value}\r\n"));
        }
        response.push_str("\r\n");
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.write_all(&response_body);
        request
    });
    (Url::parse(&format!("http://{address}/")).unwrap(), handle)
}
