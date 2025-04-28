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
        eprintln!("Creating Redis client with URL: {}", redis_url);
        let client = redis::Client::open(redis_url).map_err(|e| {
            eprintln!("Failed to create Redis client: {}", e);
            ClassifyError::StorageError(format!("Failed to create Redis client: {}", e))
        })?;

        eprintln!("Getting async connection...");
        let mut connection = match client.get_async_connection().await {
            Ok(conn) => {
                eprintln!("Redis connection established successfully");
                conn
            }
            Err(e) => {
                eprintln!("Failed to connect to Redis: {}", e);
                return Err(ClassifyError::StorageError(format!(
                    "Failed to connect to Redis: {}",
                    e
                )));
            }
        };

        if let Some(password) = redis_password {
            eprintln!("Authenticating to Redis...");
            match redis::cmd("AUTH")
                .arg(password)
                .query_async::<_, ()>(&mut connection)
                .await
            {
                Ok(_) => eprintln!("Redis authentication successful"),
                Err(e) => {
                    eprintln!("Failed to authenticate to Redis: {}", e);
                    return Err(ClassifyError::StorageError(format!(
                        "Failed to authenticate to Redis: {}",
                        e
                    )));
                }
            }
        }

        // Test the connection with a PING
        eprintln!("Testing Redis connection with PING...");
        match redis::cmd("PING")
            .query_async::<_, String>(&mut connection)
            .await
        {
            Ok(response) => eprintln!("Redis PING successful: {}", response),
            Err(e) => {
                eprintln!("Redis PING failed: {}", e);
                return Err(ClassifyError::StorageError(format!(
                    "Redis PING failed: {}",
                    e
                )));
            }
        }

        let prefix = prefix.unwrap_or("classify:content:").to_string();
        eprintln!("Using Redis prefix: {}", prefix);

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
        eprintln!("Storing content with key: {}", content_key);

        let json = match serde_json::to_string(content) {
            Ok(json) => json,
            Err(e) => {
                eprintln!("Failed to serialize content: {}", e);
                return Err(ClassifyError::SerializationError(e));
            }
        };

        let mut pipe = Pipeline::new();
        pipe.set(&content_key, &json);

        if let Some(hash) = &content.content_hash {
            let hash_index_key = self.get_hash_index_key();
            eprintln!("Adding hash index: {}={}", hash, content.id);
            pipe.hset(&hash_index_key, hash, content.id.to_string());
        }

        eprintln!("Acquiring Redis connection lock...");
        let mut conn = self.connection.lock().await;
        eprintln!("Executing Redis pipeline for content storage...");

        match pipe.query_async::<_, ()>(&mut *conn).await {
            Ok(_) => {
                eprintln!("Content stored successfully");
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to store content in Redis: {}", e);
                Err(ClassifyError::StorageError(format!(
                    "Failed to store content in Redis: {}",
                    e
                )))
            }
        }
    }

    async fn get(&self, id: &str) -> ClassifyResult<Option<Content>> {
        let content_key = self.get_content_key(id);
        eprintln!("Getting content with key: {}", content_key);

        eprintln!("Acquiring Redis connection lock...");
        let mut conn = self.connection.lock().await;
        eprintln!("Executing Redis GET...");

        let json: Option<String> = match conn.get(&content_key).await {
            Ok(json) => {
                eprintln!("Content retrieval successful");
                json
            }
            Err(e) => {
                eprintln!("Failed to get content from Redis: {}", e);
                return Err(ClassifyError::StorageError(format!(
                    "Failed to get content from Redis: {}",
                    e
                )));
            }
        };

        match json {
            Some(json_str) => match serde_json::from_str(&json_str) {
                Ok(content) => {
                    eprintln!("Content deserialized successfully");
                    Ok(Some(content))
                }
                Err(e) => {
                    eprintln!("Failed to deserialize content: {}", e);
                    Err(ClassifyError::SerializationError(e))
                }
            },
            None => {
                eprintln!("Content not found");
                Ok(None)
            }
        }
    }

    async fn list(&self) -> ClassifyResult<Vec<Content>> {
        eprintln!("Listing content with prefix pattern: {}:*", self.prefix);
        eprintln!("Acquiring Redis connection lock...");
        let mut conn = self.connection.lock().await;
        let pattern = format!("{}:*", self.prefix);

        eprintln!("Executing Redis KEYS command with pattern: {}", pattern);
        let keys: Vec<String> = match redis::cmd("KEYS")
            .arg(&pattern)
            .query_async::<_, Vec<String>>(&mut *conn)
            .await
        {
            Ok(keys) => {
                eprintln!("Found {} keys matching pattern", keys.len());
                keys
            }
            Err(e) => {
                eprintln!("Failed to list content keys: {}", e);
                return Err(ClassifyError::StorageError(format!(
                    "Failed to list content keys: {}",
                    e
                )));
            }
        };

        if keys.is_empty() {
            eprintln!("No keys found, returning empty list");
            return Ok(Vec::new());
        }

        eprintln!("Executing Redis MGET command for {} keys", keys.len());
        let json_strings: Vec<Option<String>> = match redis::cmd("MGET")
            .arg(&keys)
            .query_async::<_, Vec<Option<String>>>(&mut *conn)
            .await
        {
            Ok(strings) => {
                eprintln!("MGET successful, retrieved {} values", strings.len());
                strings
            }
            Err(e) => {
                eprintln!("Failed to get content data: {}", e);
                return Err(ClassifyError::StorageError(format!(
                    "Failed to get content data: {}",
                    e
                )));
            }
        };

        let mut contents = Vec::new();
        for json_opt in json_strings.into_iter().flatten() {
            let json_string = json_opt.clone();
            match serde_json::from_str::<Content>(&json_string) {
                Ok(content) => {
                    eprintln!("Successfully deserialized content item");
                    contents.push(content);
                }
                Err(e) => {
                    eprintln!("Error deserializing content: {}", e);
                }
            }
        }

        eprintln!("Returning {} content items", contents.len());
        Ok(contents)
    }

    async fn delete(&self, id: &str) -> ClassifyResult<bool> {
        let content_key = self.get_content_key(id);
        eprintln!("Deleting content with key: {}", content_key);

        eprintln!("Acquiring Redis connection lock...");
        let mut conn = self.connection.lock().await;

        eprintln!("Getting content before deletion");
        let json: Option<String> = match conn.get(&content_key).await {
            Ok(json) => json,
            Err(e) => {
                eprintln!("Failed to get content for deletion: {}", e);
                return Err(ClassifyError::StorageError(format!(
                    "Failed to get content for deletion: {}",
                    e
                )));
            }
        };

        let mut pipe = Pipeline::new();

        if let Some(json_str) = json {
            match serde_json::from_str::<Content>(&json_str) {
                Ok(content) => {
                    if let Some(hash) = &content.content_hash {
                        let hash_index_key = self.get_hash_index_key();
                        eprintln!("Removing hash index: {}", hash);
                        pipe.hdel(&hash_index_key, hash);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to deserialize content for deletion: {}", e);
                    return Err(ClassifyError::SerializationError(e));
                }
            }

            eprintln!("Deleting content key: {}", content_key);
            pipe.del(&content_key);

            eprintln!("Executing Redis pipeline for deletion...");
            match pipe.query_async::<_, ()>(&mut *conn).await {
                Ok(_) => {
                    eprintln!("Content deleted successfully");
                    Ok(true)
                }
                Err(e) => {
                    eprintln!("Failed to delete content: {}", e);
                    Err(ClassifyError::StorageError(format!(
                        "Failed to delete content: {}",
                        e
                    )))
                }
            }
        } else {
            eprintln!("Content not found for deletion");
            Ok(false)
        }
    }

    async fn find_by_hash(&self, hash: &str) -> ClassifyResult<Option<Content>> {
        let hash_index_key = self.get_hash_index_key();
        eprintln!(
            "Finding content by hash: {} using index: {}",
            hash, hash_index_key
        );

        eprintln!("Acquiring Redis connection lock...");
        let mut conn = self.connection.lock().await;

        eprintln!("Executing Redis HGET...");
        let content_id: Option<String> = match conn
            .hget::<_, _, Option<String>>(&hash_index_key, hash)
            .await
        {
            Ok(id) => {
                if id.is_some() {
                    eprintln!("Content ID found for hash: {:?}", id);
                } else {
                    eprintln!("No content found for hash");
                }
                id
            }
            Err(e) => {
                eprintln!("Failed to look up content by hash: {}", e);
                return Err(ClassifyError::StorageError(format!(
                    "Failed to look up content by hash: {}",
                    e
                )));
            }
        };

        // Release the connection lock before calling self.get
        // Otherwise we'll try to lock the same mutex twice, causing deadlock
        drop(conn);

        match content_id {
            Some(id) => {
                eprintln!("Retrieving content with ID: {}", id);
                self.get(&id).await
            }
            None => {
                eprintln!("No content found for hash: {}", hash);
                Ok(None)
            }
        }
    }
}
