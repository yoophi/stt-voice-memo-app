use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

use async_trait::async_trait;
use transcription_core::{
    GetOrCreateResult, OperationRepository, RepositoryError, TranscriptionOperation,
    TranscriptionOperationId,
};

pub struct LocalOperationStore {
    root: PathBuf,
    gate: Mutex<()>,
}

impl LocalOperationStore {
    pub fn new(root: PathBuf) -> Result<Self, RepositoryError> {
        fs::create_dir_all(&root).map_err(|_| RepositoryError::Unavailable)?;
        Ok(Self {
            root,
            gate: Mutex::new(()),
        })
    }

    fn path(&self, id: &TranscriptionOperationId) -> PathBuf {
        self.root.join(format!("{id}.json"))
    }

    fn load_path(path: &Path) -> Result<TranscriptionOperation, RepositoryError> {
        let bytes = fs::read(path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                RepositoryError::NotFound
            } else {
                RepositoryError::Unavailable
            }
        })?;
        serde_json::from_slice(&bytes).map_err(|_| RepositoryError::Unavailable)
    }

    fn write_atomic(&self, operation: &TranscriptionOperation) -> Result<(), RepositoryError> {
        fs::create_dir_all(&self.root).map_err(|_| RepositoryError::Unavailable)?;
        let path = self.path(operation.id());
        let temporary = path.with_extension(format!("json.tmp-{}", uuid::Uuid::new_v4()));
        let bytes = serde_json::to_vec(operation).map_err(|_| RepositoryError::Unavailable)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| RepositoryError::Unavailable)?;
        let result = (|| {
            file.write_all(&bytes)
                .map_err(|_| RepositoryError::Unavailable)?;
            file.sync_all().map_err(|_| RepositoryError::Unavailable)?;
            fs::rename(&temporary, &path).map_err(|_| RepositoryError::Unavailable)?;
            File::open(&self.root)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| RepositoryError::Unavailable)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    fn all(
        &self,
    ) -> Result<HashMap<TranscriptionOperationId, TranscriptionOperation>, RepositoryError> {
        let mut operations = HashMap::new();
        for entry in fs::read_dir(&self.root).map_err(|_| RepositoryError::Unavailable)? {
            let entry = entry.map_err(|_| RepositoryError::Unavailable)?;
            if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let operation = Self::load_path(&entry.path())?;
            operations.insert(operation.id().clone(), operation);
        }
        Ok(operations)
    }
}

#[async_trait]
impl OperationRepository for LocalOperationStore {
    async fn get_or_create(
        &self,
        candidate: TranscriptionOperation,
    ) -> Result<GetOrCreateResult, RepositoryError> {
        let _guard = self.gate.lock().map_err(|_| RepositoryError::Unavailable)?;
        if let Some(existing) = self.all()?.into_values().find(|existing| {
            existing.source_audio_id() == candidate.source_audio_id()
                && existing.fingerprint() == candidate.fingerprint()
                && existing.options() == candidate.options()
        }) {
            return Ok(GetOrCreateResult {
                operation: existing,
                created: false,
            });
        }
        self.write_atomic(&candidate)?;
        Ok(GetOrCreateResult {
            operation: candidate,
            created: true,
        })
    }

    async fn load(
        &self,
        operation_id: &TranscriptionOperationId,
    ) -> Result<TranscriptionOperation, RepositoryError> {
        let _guard = self.gate.lock().map_err(|_| RepositoryError::Unavailable)?;
        Self::load_path(&self.path(operation_id))
    }

    async fn compare_and_swap(
        &self,
        expected_revision: u64,
        mut replacement: TranscriptionOperation,
    ) -> Result<TranscriptionOperation, RepositoryError> {
        let _guard = self.gate.lock().map_err(|_| RepositoryError::Unavailable)?;
        let current = Self::load_path(&self.path(replacement.id()))?;
        if current.revision() != expected_revision {
            return Err(RepositoryError::RevisionConflict);
        }
        replacement.set_revision(expected_revision.saturating_add(1));
        self.write_atomic(&replacement)?;
        Ok(replacement)
    }

    async fn list_unfinished(&self) -> Result<Vec<TranscriptionOperation>, RepositoryError> {
        let _guard = self.gate.lock().map_err(|_| RepositoryError::Unavailable)?;
        Ok(self
            .all()?
            .into_values()
            .filter(|operation| operation.terminal_winner().is_none())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use transcription_core::{
        OperationRepository, SourceAudioId, SourceDescriptor, SubmissionFingerprint,
        TranscriptionOperation, TranscriptionOperationId, TranscriptionOptions,
    };

    use super::*;

    fn operation() -> TranscriptionOperation {
        let source = SourceDescriptor::new(
            SourceAudioId::parse("source-one").unwrap(),
            "audio/mp4",
            "m4a",
            64,
            1_000,
            "a".repeat(64),
        )
        .unwrap();
        let options = TranscriptionOptions::default();
        TranscriptionOperation::new(
            TranscriptionOperationId::new(),
            source.id.clone(),
            SubmissionFingerprint::derive(&source, &options),
            options,
        )
    }

    #[tokio::test]
    async fn persists_atomically_and_detects_revision_conflicts() {
        let directory = tempdir().unwrap();
        let store = LocalOperationStore::new(directory.path().to_path_buf()).unwrap();
        let first = store.get_or_create(operation()).await.unwrap().operation;
        let committed = store.compare_and_swap(0, first.clone()).await.unwrap();
        assert_eq!(committed.revision(), 1);
        assert_eq!(
            store.compare_and_swap(0, first).await.unwrap_err(),
            RepositoryError::RevisionConflict
        );
        assert_eq!(store.list_unfinished().await.unwrap().len(), 1);
        let raw = fs::read_to_string(store.path(committed.id())).unwrap();
        for forbidden in ["transcript", "authorization", "storagePath", "fileUri"] {
            assert!(!raw.contains(forbidden));
        }
    }
}
