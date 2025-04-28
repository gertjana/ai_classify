use crate::storage::ContentStorage;
use crate::{ClassifyError, ClassifyResult, Content};
use std::env;
use std::time::Duration;
use tokio::time::timeout;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::content::redis::RedisContentStorage;
    use uuid::Uuid;

    #[tokio::test]
    #[ignore]
    async fn test_redis_storage_integration() -> ClassifyResult<()> {
        // Set a timeout for operations (5 seconds)
        let op_timeout = Duration::from_secs(5);

        println!("========== STARTING REDIS TEST ==========");

        // This test requires a Redis server
        // It's marked as 'ignore' so it doesn't run in normal test runs
        let redis_url =
            env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let redis_password = env::var("TEST_REDIS_PASSWORD").ok();
        let prefix = format!("test:{}:", Uuid::new_v4());

        println!(
            "Connecting to Redis at {} with prefix {}",
            redis_url, prefix
        );

        // Create the Redis client and test the connection before proceeding
        let client = redis::Client::open(redis_url.as_str()).map_err(|e| {
            println!("Failed to create Redis client: {}", e);
            ClassifyError::StorageError(format!("Failed to create Redis client: {}", e))
        })?;

        println!("Getting async connection...");
        let conn_future = client.get_async_connection();
        let mut conn = match timeout(op_timeout, conn_future).await {
            Ok(result) => result.map_err(|e| {
                println!("Failed to connect to Redis: {}", e);
                ClassifyError::StorageError(format!("Failed to connect to Redis: {}", e))
            })?,
            Err(_) => {
                println!("Timeout while connecting to Redis");
                return Err(ClassifyError::StorageError(
                    "Timeout while connecting to Redis".to_string(),
                ));
            }
        };

        // Test a simple ping command to verify connection
        println!("Sending PING command...");
        let cmd = redis::cmd("PING");
        let ping_future = cmd.query_async::<_, String>(&mut conn);
        match timeout(op_timeout, ping_future).await {
            Ok(result) => result.map_err(|e| {
                println!("Redis PING failed: {}", e);
                ClassifyError::StorageError(format!("Redis PING failed: {}", e))
            })?,
            Err(_) => {
                println!("Timeout during Redis PING");
                return Err(ClassifyError::StorageError(
                    "Timeout during Redis PING".to_string(),
                ));
            }
        };

        println!("Successfully connected to Redis and verified with PING");

        println!("Creating RedisContentStorage instance...");
        let storage_future =
            RedisContentStorage::new(&redis_url, redis_password.as_deref(), Some(&prefix));
        let storage = match timeout(op_timeout, storage_future).await {
            Ok(result) => result?,
            Err(_) => {
                println!("Timeout while creating RedisContentStorage");
                return Err(ClassifyError::StorageError(
                    "Timeout while creating RedisContentStorage".to_string(),
                ));
            }
        };

        println!("Creating test content");
        let content = Content::new("Redis storage test content".to_string())
            .with_tags(vec!["test".to_string(), "redis".to_string()]);
        let content_id = content.id.to_string();
        println!("Content created with ID: {}", content_id);

        println!("Storing content...");
        let store_future = storage.store(&content);
        match timeout(op_timeout, store_future).await {
            Ok(result) => result?,
            Err(_) => {
                println!("Timeout while storing content");
                return Err(ClassifyError::StorageError(
                    "Timeout while storing content".to_string(),
                ));
            }
        };
        println!("Content stored successfully");

        println!("Retrieving content...");
        let get_future = storage.get(&content_id);
        let retrieved = match timeout(op_timeout, get_future).await {
            Ok(result) => result?,
            Err(_) => {
                println!("Timeout while retrieving content");
                return Err(ClassifyError::StorageError(
                    "Timeout while retrieving content".to_string(),
                ));
            }
        };

        assert!(retrieved.is_some(), "Content should be retrieved");
        let retrieved = retrieved.unwrap();
        assert_eq!(
            retrieved.id, content.id,
            "Retrieved content ID should match"
        );
        assert_eq!(
            retrieved.content, content.content,
            "Retrieved content text should match"
        );
        assert_eq!(
            retrieved.tags, content.tags,
            "Retrieved content tags should match"
        );
        println!("Content retrieved successfully");

        println!("Listing contents...");
        let list_future = storage.list();
        let contents = match timeout(op_timeout, list_future).await {
            Ok(result) => result?,
            Err(_) => {
                println!("Timeout while listing contents");
                return Err(ClassifyError::StorageError(
                    "Timeout while listing contents".to_string(),
                ));
            }
        };

        assert_eq!(contents.len(), 1, "Should have exactly one content item");
        println!("Content list successful, found {} items", contents.len());

        println!("Finding content by hash...");
        let hash = content.content_hash.as_ref().unwrap();
        let find_future = storage.find_by_hash(hash);
        let found = match timeout(op_timeout, find_future).await {
            Ok(result) => result?,
            Err(_) => {
                println!("Timeout while finding content by hash");
                return Err(ClassifyError::StorageError(
                    "Timeout while finding content by hash".to_string(),
                ));
            }
        };

        assert!(found.is_some(), "Content should be found by hash");
        assert_eq!(
            found.unwrap().id,
            content.id,
            "Found content ID should match"
        );
        println!("Content found by hash successfully");

        println!("Deleting content...");
        let delete_future = storage.delete(&content_id);
        let deleted = match timeout(op_timeout, delete_future).await {
            Ok(result) => result?,
            Err(_) => {
                println!("Timeout while deleting content");
                return Err(ClassifyError::StorageError(
                    "Timeout while deleting content".to_string(),
                ));
            }
        };

        assert!(deleted, "Content should be deleted");
        println!("Content deleted successfully");

        println!("Verifying deletion...");
        let verify_future = storage.get(&content_id);
        let retrieved = match timeout(op_timeout, verify_future).await {
            Ok(result) => result?,
            Err(_) => {
                println!("Timeout while verifying deletion");
                return Err(ClassifyError::StorageError(
                    "Timeout while verifying deletion".to_string(),
                ));
            }
        };

        assert!(
            retrieved.is_none(),
            "Content should not exist after deletion"
        );
        println!("Deletion verified successfully");

        let delete_again_future = storage.delete(&content_id);
        let deleted = match timeout(op_timeout, delete_again_future).await {
            Ok(result) => result?,
            Err(_) => {
                println!("Timeout while trying to delete non-existent content");
                return Err(ClassifyError::StorageError(
                    "Timeout while trying to delete non-existent content".to_string(),
                ));
            }
        };

        assert!(
            !deleted,
            "Deleting non-existent content should return false"
        );
        println!("Deletion of non-existent content returned false as expected");

        println!("Test completed successfully");
        Ok(())
    }
}
