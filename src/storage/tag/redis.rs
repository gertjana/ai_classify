use async_trait::async_trait;
use redis::AsyncCommands;
use std::collections::HashSet;
use std::sync::Arc;

use crate::storage::TagStorage;
use crate::{ClassifyError, ClassifyResult};

/// Redis-based tag storage
pub struct RedisTagStorage {
    connection: Arc<tokio::sync::Mutex<redis::aio::Connection>>,
}

impl RedisTagStorage {
    pub async fn new(redis_url: &str, redis_password: Option<&str>) -> ClassifyResult<Self> {
        let client = redis::Client::open(redis_url).map_err(|e| {
            ClassifyError::StorageError(format!("Failed to create Redis client: {}", e))
        })?;

        let mut connection = client.get_async_connection().await.map_err(|e| {
            ClassifyError::StorageError(format!("Failed to connect to Redis: {}", e))
        })?;

        if let Some(password) = redis_password {
            redis::cmd("AUTH")
                .arg(password)
                .query_async::<_, ()>(&mut connection)
                .await
                .map_err(|e| {
                    ClassifyError::StorageError(format!("Failed to authenticate to Redis: {}", e))
                })?;
        }

        Ok(Self {
            connection: Arc::new(tokio::sync::Mutex::new(connection)),
        })
    }

    fn get_content_tags_key(&self, content_id: &str) -> String {
        format!("classify:content:{}:tags", content_id)
    }

    fn get_tag_contents_key(&self, tag: &str) -> String {
        format!("classify:tag:{}:contents", tag)
    }

    fn get_all_tag_contents_pattern(&self) -> String {
        "classify:tag:*:contents".to_string()
    }
}

#[async_trait]
impl TagStorage for RedisTagStorage {
    async fn add_tags(&self, content_id: &str, tags: &[String]) -> ClassifyResult<()> {
        let mut conn = self.connection.lock().await;
        let content_tags_key = self.get_content_tags_key(content_id);

        let mut pipe = redis::pipe();

        for tag in tags {
            pipe.sadd(&content_tags_key, tag);

            let tag_contents_key = self.get_tag_contents_key(tag);
            pipe.sadd(&tag_contents_key, content_id);
        }

        pipe.query_async::<_, ()>(&mut *conn)
            .await
            .map_err(|e| ClassifyError::StorageError(format!("Failed to add tags: {}", e)))?;

        Ok(())
    }

    async fn get_tags(&self, content_id: &str) -> ClassifyResult<Vec<String>> {
        let mut conn = self.connection.lock().await;
        let content_tags_key = self.get_content_tags_key(content_id);

        let tags: Vec<String> = conn
            .smembers(&content_tags_key)
            .await
            .map_err(|e| ClassifyError::StorageError(format!("Failed to get tags: {}", e)))?;

        Ok(tags)
    }

    async fn list_tags(&self) -> ClassifyResult<Vec<String>> {
        let mut conn = self.connection.lock().await;
        let pattern = self.get_all_tag_contents_pattern();

        let tag_keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut *conn)
            .await
            .map_err(|e| ClassifyError::StorageError(format!("Failed to list tag keys: {}", e)))?;

        let mut tags = HashSet::new();
        for key in tag_keys {
            if let Some(tag) = key
                .strip_prefix("classify:tag:")
                .and_then(|s| s.strip_suffix(":contents"))
            {
                tags.insert(tag.to_string());
            }
        }

        Ok(tags.into_iter().collect())
    }

    async fn find_by_tag(&self, tag: &str) -> ClassifyResult<Vec<String>> {
        let mut conn = self.connection.lock().await;
        let tag_contents_key = self.get_tag_contents_key(tag);

        let content_ids: Vec<String> = conn
            .smembers(&tag_contents_key)
            .await
            .map_err(|e| ClassifyError::StorageError(format!("Failed to find by tag: {}", e)))?;

        Ok(content_ids)
    }

    async fn remove_tags(&self, content_id: &str, tags: &[String]) -> ClassifyResult<()> {
        let mut conn = self.connection.lock().await;
        let content_tags_key = self.get_content_tags_key(content_id);

        let mut pipe = redis::pipe();

        for tag in tags {
            pipe.srem(&content_tags_key, tag);

            let tag_contents_key = self.get_tag_contents_key(tag);
            pipe.srem(&tag_contents_key, content_id);

            pipe.exists(&tag_contents_key);
        }

        pipe.query_async::<_, ()>(&mut *conn)
            .await
            .map_err(|e| ClassifyError::StorageError(format!("Failed to remove tags: {}", e)))?;

        Ok(())
    }
}
