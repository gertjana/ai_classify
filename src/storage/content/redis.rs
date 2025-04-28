use async_trait::async_trait;
use redis::{AsyncCommands, Pipeline};
use std::sync::Arc;

use crate::storage::ContentStorage;
use crate::{ClassifyError, ClassifyResult, Content};

/// Redis-based content storage
pub struct RedisContentStorage {
    connection: Arc<tokio::sync::Mutex<redis::aio::Connection>>,
    prefix: String,
}

impl RedisContentStorage {
    pub async fn new(
        redis_url: &str,
        redis_password: Option<&str>,
        prefix: Option<&str>,
    ) -> ClassifyResult<Self> {
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

        let prefix = prefix.unwrap_or("classify:content:").to_string();

        Ok(Self {
            connection: Arc::new(tokio::sync::Mutex::new(connection)),
            prefix,
        })
    }

    fn get_content_key(&self, id: &str) -> String {
        format!("{}:{}", self.prefix, id)
    }

    fn get_hash_index_key(&self) -> String {
        format!("{}hash_index", self.prefix)
    }
}

#[async_trait]
impl ContentStorage for RedisContentStorage {
    async fn store(&self, content: &Content) -> ClassifyResult<()> {
        let content_key = self.get_content_key(&content.id.to_string());
        let json = serde_json::to_string(content).map_err(ClassifyError::SerializationError)?;

        let mut pipe = Pipeline::new();

        pipe.set(&content_key, &json);

        if let Some(hash) = &content.content_hash {
            let hash_index_key = self.get_hash_index_key();
            pipe.hset(&hash_index_key, hash, content.id.to_string());
        }

        let mut conn = self.connection.lock().await;
        pipe.query_async::<_, ()>(&mut *conn).await.map_err(|e| {
            ClassifyError::StorageError(format!("Failed to store content in Redis: {}", e))
        })?;

        Ok(())
    }

    async fn get(&self, id: &str) -> ClassifyResult<Option<Content>> {
        let content_key = self.get_content_key(id);
        let mut conn = self.connection.lock().await;

        let json: Option<String> = conn.get(&content_key).await.map_err(|e| {
            ClassifyError::StorageError(format!("Failed to get content from Redis: {}", e))
        })?;

        match json {
            Some(json_str) => {
                let content =
                    serde_json::from_str(&json_str).map_err(ClassifyError::SerializationError)?;
                Ok(Some(content))
            }
            None => Ok(None),
        }
    }

    async fn list(&self) -> ClassifyResult<Vec<Content>> {
        let mut conn = self.connection.lock().await;
        let pattern = format!("{}:*", self.prefix);

        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut *conn)
            .await
            .map_err(|e| {
                ClassifyError::StorageError(format!("Failed to list content keys: {}", e))
            })?;

        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let json_strings: Vec<Option<String>> = conn.get(keys).await.map_err(|e| {
            ClassifyError::StorageError(format!("Failed to get content data: {}", e))
        })?;

        let mut contents = Vec::new();
        for json_opt in json_strings.into_iter().flatten() {
            match serde_json::from_str(&json_opt) {
                Ok(content) => contents.push(content),
                Err(e) => {
                    eprintln!("Error deserializing content: {}", e);
                }
            }
        }

        Ok(contents)
    }

    async fn delete(&self, id: &str) -> ClassifyResult<bool> {
        let content_key = self.get_content_key(id);
        let mut conn = self.connection.lock().await;

        let json: Option<String> = conn.get(&content_key).await.map_err(|e| {
            ClassifyError::StorageError(format!("Failed to get content for deletion: {}", e))
        })?;

        let mut pipe = Pipeline::new();

        if let Some(json_str) = json {
            match serde_json::from_str::<Content>(&json_str) {
                Ok(content) => {
                    if let Some(hash) = &content.content_hash {
                        let hash_index_key = self.get_hash_index_key();
                        pipe.hdel(&hash_index_key, hash);
                    }
                }
                Err(e) => {
                    return Err(ClassifyError::SerializationError(e));
                }
            }

            pipe.del(&content_key);

            pipe.query_async::<_, ()>(&mut *conn).await.map_err(|e| {
                ClassifyError::StorageError(format!("Failed to delete content: {}", e))
            })?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn find_by_hash(&self, hash: &str) -> ClassifyResult<Option<Content>> {
        let hash_index_key = self.get_hash_index_key();
        let mut conn = self.connection.lock().await;

        let content_id: Option<String> = conn.hget(&hash_index_key, hash).await.map_err(|e| {
            ClassifyError::StorageError(format!("Failed to look up content by hash: {}", e))
        })?;

        match content_id {
            Some(id) => self.get(&id).await,
            None => Ok(None),
        }
    }
}
