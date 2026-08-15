use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::RwLock,
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use transcription_core::{SourceAudioError, SourceAudioId, SourceAudioPort, SourceDescriptor};

use super::atomic_file;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceManifest {
    relative_path: PathBuf,
    media_type: String,
    file_extension: String,
    byte_length: u64,
    duration_ms: u64,
    sha256: String,
}

pub struct PrivateSourceAudioStore {
    root: PathBuf,
    manifest_root: PathBuf,
    manifests: RwLock<HashMap<SourceAudioId, SourceManifest>>,
}

impl PrivateSourceAudioStore {
    pub fn new(root: PathBuf) -> Result<Self, SourceAudioError> {
        fs::create_dir_all(&root).map_err(|_| SourceAudioError::Unavailable)?;
        let manifest_root = root.join(".manifests");
        fs::create_dir_all(&manifest_root).map_err(|_| SourceAudioError::Unavailable)?;
        Ok(Self {
            root,
            manifest_root,
            manifests: RwLock::new(HashMap::new()),
        })
    }

    #[cfg(any(test, debug_assertions))]
    #[allow(dead_code)]
    pub fn register_fixture(
        &self,
        source_id: SourceAudioId,
        relative_path: PathBuf,
        media_type: impl Into<String>,
        duration_ms: u64,
    ) -> Result<SourceDescriptor, SourceAudioError> {
        let path = self.resolve_contained(&relative_path)?;
        let bytes = fs::read(&path).map_err(|_| SourceAudioError::NotFound)?;
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .ok_or(SourceAudioError::Invalid)?;
        let sha256 = hex_digest(&bytes);
        let manifest = SourceManifest {
            relative_path,
            media_type: media_type.into(),
            file_extension: extension.to_owned(),
            byte_length: bytes.len() as u64,
            duration_ms,
            sha256,
        };
        self.persist_manifest(&source_id, &manifest)?;
        self.manifests
            .write()
            .map_err(|_| SourceAudioError::Unavailable)?
            .insert(source_id.clone(), manifest);
        self.inspect_sync(&source_id)
    }

    pub fn resolve_path(&self, source_id: &SourceAudioId) -> Result<PathBuf, SourceAudioError> {
        let manifest = self.load_manifest(source_id)?;
        self.resolve_contained(&manifest.relative_path)
    }

    fn inspect_sync(
        &self,
        source_id: &SourceAudioId,
    ) -> Result<SourceDescriptor, SourceAudioError> {
        let manifest = self.load_manifest(source_id)?;
        let path = self.resolve_contained(&manifest.relative_path)?;
        let bytes = fs::read(&path).map_err(|_| SourceAudioError::NotFound)?;
        if bytes.len() as u64 != manifest.byte_length || hex_digest(&bytes) != manifest.sha256 {
            return Err(SourceAudioError::Invalid);
        }
        SourceDescriptor::new(
            source_id.clone(),
            manifest.media_type,
            manifest.file_extension,
            manifest.byte_length,
            manifest.duration_ms,
            manifest.sha256,
        )
        .map_err(|_| SourceAudioError::Invalid)
    }

    fn manifest_path(&self, source_id: &SourceAudioId) -> PathBuf {
        self.manifest_root.join(format!(
            "{}.json",
            hex_digest(source_id.as_str().as_bytes())
        ))
    }

    fn load_manifest(&self, source_id: &SourceAudioId) -> Result<SourceManifest, SourceAudioError> {
        if let Some(manifest) = self
            .manifests
            .read()
            .map_err(|_| SourceAudioError::Unavailable)?
            .get(source_id)
            .cloned()
        {
            return Ok(manifest);
        }
        let bytes = fs::read(self.manifest_path(source_id)).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                SourceAudioError::NotFound
            } else {
                SourceAudioError::Unavailable
            }
        })?;
        let manifest: SourceManifest =
            serde_json::from_slice(&bytes).map_err(|_| SourceAudioError::Invalid)?;
        self.manifests
            .write()
            .map_err(|_| SourceAudioError::Unavailable)?
            .insert(source_id.clone(), manifest.clone());
        Ok(manifest)
    }

    fn persist_manifest(
        &self,
        source_id: &SourceAudioId,
        manifest: &SourceManifest,
    ) -> Result<(), SourceAudioError> {
        let destination = self.manifest_path(source_id);
        let bytes = serde_json::to_vec(manifest).map_err(|_| SourceAudioError::Unavailable)?;
        atomic_file::replace(&destination, &bytes).map_err(|_| SourceAudioError::Unavailable)
    }

    fn resolve_contained(&self, relative: &Path) -> Result<PathBuf, SourceAudioError> {
        if relative.is_absolute()
            || relative
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err(SourceAudioError::Invalid);
        }
        let candidate = self.root.join(relative);
        let canonical_root = self
            .root
            .canonicalize()
            .map_err(|_| SourceAudioError::Unavailable)?;
        let canonical = candidate
            .canonicalize()
            .map_err(|_| SourceAudioError::NotFound)?;
        if !canonical.starts_with(canonical_root) {
            return Err(SourceAudioError::Invalid);
        }
        Ok(canonical)
    }
}

#[async_trait]
impl SourceAudioPort for PrivateSourceAudioStore {
    async fn inspect(
        &self,
        source_id: &SourceAudioId,
    ) -> Result<SourceDescriptor, SourceAudioError> {
        self.inspect_sync(source_id)
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use transcription_core::SourceAudioPort;

    use super::*;

    #[tokio::test]
    async fn validates_containment_and_detects_changed_source() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("fixture.m4a"), b"safe fixture").unwrap();
        let store = PrivateSourceAudioStore::new(root.path().to_path_buf()).unwrap();
        let id = SourceAudioId::parse("fixture-source").unwrap();
        store
            .register_fixture(id.clone(), PathBuf::from("fixture.m4a"), "audio/mp4", 1_000)
            .unwrap();
        assert!(store.inspect(&id).await.is_ok());
        let reopened = PrivateSourceAudioStore::new(root.path().to_path_buf()).unwrap();
        assert!(reopened.inspect(&id).await.is_ok());
        fs::write(root.path().join("fixture.m4a"), b"changed").unwrap();
        assert_eq!(
            store.inspect(&id).await.unwrap_err(),
            SourceAudioError::Invalid
        );
        assert!(
            store
                .register_fixture(
                    SourceAudioId::parse("escape").unwrap(),
                    PathBuf::from("../escape.m4a"),
                    "audio/mp4",
                    1_000,
                )
                .is_err()
        );
    }
}
