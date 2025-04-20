pub mod content;
pub mod tag;

#[cfg(test)]
mod integration_test;

use async_trait::async_trait;
use crate::{ClassifyResult, Content};
use std::sync::Arc;

/// ContentStorage trait for storing and retrieving content
#[async_trait]
pub trait ContentStorage: Send + Sync {
    /// Store content
    async fn store(&self, content: &Content) -> ClassifyResult<()>;

    /// Retrieve content by ID
    async fn get(&self, id: &str) -> ClassifyResult<Option<Content>>;

    /// List all content
    async fn list(&self) -> ClassifyResult<Vec<Content>>;

    /// Delete content by ID
    async fn delete(&self, id: &str) -> ClassifyResult<bool>;
}

/// TagStorage trait for storing and retrieving tags
#[async_trait]
pub trait TagStorage: Send + Sync {
    /// Add tags to content
    async fn add_tags(&self, content_id: &str, tags: &[String]) -> ClassifyResult<()>;

    /// Get tags for content
    async fn get_tags(&self, content_id: &str) -> ClassifyResult<Vec<String>>;

    /// List all tags
    async fn list_tags(&self) -> ClassifyResult<Vec<String>>;

    /// Find content by tag
    async fn find_by_tag(&self, tag: &str) -> ClassifyResult<Vec<String>>;

    /// Remove tags from content
    async fn remove_tags(&self, content_id: &str, tags: &[String]) -> ClassifyResult<()>;
}

/// Content storage factory
pub async fn create_content_storage(
    storage_type: &crate::config::StorageType,
    config: &crate::config::StorageConfig,
) -> ClassifyResult<Arc<dyn ContentStorage>> {
    match storage_type {
        crate::config::StorageType::Filesystem => {
            let storage = content::filesystem::FilesystemContentStorage::new(&config.content_storage_path)?;
            Ok(Arc::new(storage))
        }
        // Add more storage types as needed
    }
}

/// Tag storage factory
pub async fn create_tag_storage(
    storage_type: &crate::config::TagStorageType,
    config: &crate::config::TagStorageConfig,
) -> ClassifyResult<Arc<dyn TagStorage>> {
    match storage_type {
        crate::config::TagStorageType::Redis => {
            let storage = tag::redis::RedisTagStorage::new(&config.redis_url, config.redis_password.as_deref()).await?;
            Ok(Arc::new(storage))
        }
        // Add more tag storage types as needed
    }
}
