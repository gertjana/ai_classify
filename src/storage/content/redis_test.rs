use crate::storage::ContentStorage;
use crate::{ClassifyResult, Content};
use std::env;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::content::redis::RedisContentStorage;
    use uuid::Uuid;

    #[tokio::test]
    #[ignore]
    async fn test_redis_storage_integration() -> ClassifyResult<()> {
        // This test requires a Redis server
        // It's marked as 'ignore' so it doesn't run in normal test runs

        let redis_url =
            env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let redis_password = env::var("TEST_REDIS_PASSWORD").ok();
        let prefix = format!("test:{}:", Uuid::new_v4());

        let storage =
            RedisContentStorage::new(&redis_url, redis_password.as_deref(), Some(&prefix)).await?;

        let content = Content::new("Redis storage test content".to_string())
            .with_tags(vec!["test".to_string(), "redis".to_string()]);
        let content_id = content.id.to_string();

        storage.store(&content).await?;

        let retrieved = storage.get(&content_id).await?;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, content.id);
        assert_eq!(retrieved.content, content.content);
        assert_eq!(retrieved.tags, content.tags);

        let contents = storage.list().await?;
        assert_eq!(contents.len(), 1);

        let hash = content.content_hash.as_ref().unwrap();
        let found = storage.find_by_hash(hash).await?;
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, content.id);

        let deleted = storage.delete(&content_id).await?;
        assert!(deleted);

        let retrieved = storage.get(&content_id).await?;
        assert!(retrieved.is_none());

        let deleted = storage.delete(&content_id).await?;
        assert!(!deleted);

        Ok(())
    }
}
