use async_trait::async_trait;
use std::fs;
use std::path::PathBuf;
use tokio::fs::{create_dir_all, read_dir, remove_file};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::storage::ContentStorage;
use crate::{ClassifyError, ClassifyResult, Content};

/// Filesystem-based content storage
pub struct FilesystemContentStorage {
    base_dir: PathBuf,
}

impl FilesystemContentStorage {
    pub fn new(base_dir: &str) -> ClassifyResult<Self> {
        let path = PathBuf::from(base_dir);

        fs::create_dir_all(&path).map_err(|e| {
            ClassifyError::StorageError(format!("Failed to create directory: {}", e))
        })?;

        Ok(Self { base_dir: path })
    }

    fn get_file_path(&self, id: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json", id))
    }
}

#[async_trait]
impl ContentStorage for FilesystemContentStorage {
    async fn store(&self, content: &Content) -> ClassifyResult<()> {
        let file_path = self.get_file_path(&content.id.to_string());
        let json =
            serde_json::to_string_pretty(content).map_err(ClassifyError::SerializationError)?;

        if let Some(parent) = file_path.parent() {
            create_dir_all(parent).await.map_err(|e| {
                ClassifyError::StorageError(format!("Failed to create directory: {}", e))
            })?;
        }

        let mut file = tokio::fs::File::create(&file_path)
            .await
            .map_err(|e| ClassifyError::StorageError(format!("Failed to create file: {}", e)))?;

        file.write_all(json.as_bytes())
            .await
            .map_err(|e| ClassifyError::StorageError(format!("Failed to write file: {}", e)))?;

        Ok(())
    }

    async fn get(&self, id: &str) -> ClassifyResult<Option<Content>> {
        let file_path = self.get_file_path(id);

        if !file_path.exists() {
            return Ok(None);
        }

        let mut file = match tokio::fs::File::open(&file_path).await {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                return Err(ClassifyError::StorageError(format!(
                    "Failed to open file: {}",
                    e
                )))
            }
        };

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .await
            .map_err(|e| ClassifyError::StorageError(format!("Failed to read file: {}", e)))?;

        let content = serde_json::from_str(&contents).map_err(ClassifyError::SerializationError)?;

        Ok(Some(content))
    }

    async fn list(&self) -> ClassifyResult<Vec<Content>> {
        let mut contents = Vec::new();

        let mut entries = read_dir(&self.base_dir)
            .await
            .map_err(|e| ClassifyError::StorageError(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            ClassifyError::StorageError(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                let file_name = path.file_stem().unwrap().to_string_lossy();

                if let Some(content) = self.get(&file_name).await? {
                    contents.push(content);
                }
            }
        }

        Ok(contents)
    }

    async fn delete(&self, id: &str) -> ClassifyResult<bool> {
        let file_path = self.get_file_path(id);

        if !file_path.exists() {
            return Ok(false);
        }

        remove_file(&file_path)
            .await
            .map_err(|e| ClassifyError::StorageError(format!("Failed to delete file: {}", e)))?;

        Ok(true)
    }

    async fn find_by_hash(&self, hash: &str) -> ClassifyResult<Option<Content>> {
        let all_content = self.list().await?;

        for content in all_content {
            if content.content_hash.as_deref() == Some(hash) {
                return Ok(Some(content));
            }
        }

        Ok(None)
    }
}
