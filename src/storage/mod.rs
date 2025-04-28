pub mod content;
pub mod tag;

#[cfg(test)]
mod integration_test;

use crate::{ClassifyError, ClassifyResult, Content};
use async_trait::async_trait;
use std::sync::Arc;

/// ContentStorage trait for storing and retrieving content
#[async_trait]
pub trait ContentStorage: Send + Sync {
    async fn store(&self, content: &Content) -> ClassifyResult<()>;
    async fn get(&self, id: &str) -> ClassifyResult<Option<Content>>;
    async fn list(&self) -> ClassifyResult<Vec<Content>>;
    async fn delete(&self, id: &str) -> ClassifyResult<bool>;
    async fn find_by_hash(&self, hash: &str) -> ClassifyResult<Option<Content>>;
}

/// TagStorage trait for storing and retrieving tags
#[async_trait]
pub trait TagStorage: Send + Sync {
    async fn add_tags(&self, content_id: &str, tags: &[String]) -> ClassifyResult<()>;
    async fn get_tags(&self, content_id: &str) -> ClassifyResult<Vec<String>>;
    async fn list_tags(&self) -> ClassifyResult<Vec<String>>;
    async fn find_by_tag(&self, tag: &str) -> ClassifyResult<Vec<String>>;
    async fn remove_tags(&self, content_id: &str, tags: &[String]) -> ClassifyResult<()>;
}

/// Content storage factory
pub async fn create_content_storage(
    storage_type: &crate::config::StorageType,
    config: &crate::config::StorageConfig,
) -> ClassifyResult<Arc<dyn ContentStorage>> {
    match storage_type {
        crate::config::StorageType::Filesystem => {
            let storage =
                content::filesystem::FilesystemContentStorage::new(&config.content_storage_path)?;
            Ok(Arc::new(storage))
        }
        crate::config::StorageType::Redis => {
            // Get the Redis URL, using the tag storage Redis URL as a fallback
            let redis_url = config.redis_url.as_deref().ok_or_else(|| {
                ClassifyError::ConfigError(
                    "CONTENT_REDIS_URL is required for Redis storage".to_string(),
                )
            })?;

            // Create Redis content storage
            let storage = content::redis::RedisContentStorage::new(
                redis_url,
                config.redis_password.as_deref(),
                config.redis_prefix.as_deref(),
            )
            .await?;

            Ok(Arc::new(storage))
        }
        crate::config::StorageType::S3 => {
            // Validate S3 configuration
            let bucket = config.s3_bucket.as_deref().ok_or_else(|| {
                ClassifyError::ConfigError("S3_BUCKET is required for S3 storage".to_string())
            })?;

            let region = config.s3_region.as_deref().ok_or_else(|| {
                ClassifyError::ConfigError("S3_REGION is required for S3 storage".to_string())
            })?;

            // Prefix is optional, default to empty string
            let prefix = config.s3_prefix.as_deref().unwrap_or("");

            // Create S3 content storage with appropriate authentication
            let storage = content::s3::S3ContentStorage::new(
                bucket,
                prefix,
                region,
                config.s3_profile.as_deref(),
                config.s3_access_key.as_deref(),
                config.s3_secret_key.as_deref(),
            )
            .await?;

            Ok(Arc::new(storage))
        }
    }
}

/// Tag storage factory
pub async fn create_tag_storage(
    storage_type: &crate::config::TagStorageType,
    config: &crate::config::TagStorageConfig,
) -> ClassifyResult<Arc<dyn TagStorage>> {
    match storage_type {
        crate::config::TagStorageType::Redis => {
            let storage = tag::redis::RedisTagStorage::new(
                &config.redis_url,
                config.redis_password.as_deref(),
            )
            .await?;
            Ok(Arc::new(storage))
        } // Add more tag storage types as needed
    }
}
