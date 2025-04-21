use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use crate::storage::content::filesystem::FilesystemContentStorage;
use crate::storage::tag::redis::RedisTagStorage;
use crate::storage::{ContentStorage, TagStorage};
use crate::ClassifyResult;
use crate::Content;

/// Integration tests that combine multiple storage components
/// These tests require an actual Redis server, so they are marked as ignored by default
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use mockall::predicate::*;

    mock! {
        pub TagStorageMock {}

        #[async_trait::async_trait]
        impl TagStorage for TagStorageMock {
            async fn add_tags(&self, content_id: &str, tags: &[String]) -> ClassifyResult<()>;
            async fn get_tags(&self, content_id: &str) -> ClassifyResult<Vec<String>>;
            async fn list_tags(&self) -> ClassifyResult<Vec<String>>;
            async fn find_by_tag(&self, tag: &str) -> ClassifyResult<Vec<String>>;
            async fn remove_tags(&self, content_id: &str, tags: &[String]) -> ClassifyResult<()>;
        }
    }

    fn setup_test_dir() -> PathBuf {
        let test_dir = PathBuf::from(format!("./test_data_{}", Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();
        test_dir
    }

    fn cleanup_test_dir(path: PathBuf) {
        fs::remove_dir_all(path).ok();
    }

    #[tokio::test]
    async fn test_content_with_tags() -> ClassifyResult<()> {
        let test_dir = setup_test_dir();

        let content_storage = Arc::new(FilesystemContentStorage::new(test_dir.to_str().unwrap())?);

        let mut tag_storage = MockTagStorageMock::new();

        let content_text = "Test content with tags".to_string();
        let tags = vec!["tag1".to_string(), "tag2".to_string()];
        let content = Content::new(content_text.clone()).with_tags(tags.clone());
        let content_id = content.id.to_string();

        let content_id_clone = content_id.clone();
        tag_storage
            .expect_add_tags()
            .with(
                function(move |id: &str| id == content_id_clone),
                function(|t: &[String]| {
                    t.len() == 2
                        && t.contains(&"tag1".to_string())
                        && t.contains(&"tag2".to_string())
                }),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        let tags_clone = tags.clone();
        let content_id_for_get = content_id.clone();
        tag_storage
            .expect_get_tags()
            .with(function(move |id: &str| id == content_id_for_get))
            .times(1)
            .returning(move |_| Ok(tags_clone.clone()));

        content_storage.store(&content).await?;

        tag_storage.add_tags(&content_id, &tags).await?;

        let retrieved = content_storage.get(&content_id).await?;
        assert!(retrieved.is_some());

        let retrieved_tags = tag_storage.get_tags(&content_id).await?;

        assert_eq!(retrieved_tags.len(), 2);
        assert!(retrieved_tags.contains(&"tag1".to_string()));
        assert!(retrieved_tags.contains(&"tag2".to_string()));

        cleanup_test_dir(test_dir);

        Ok(())
    }

    #[tokio::test]
    async fn test_find_by_tag_and_get_content() -> ClassifyResult<()> {
        let test_dir = setup_test_dir();

        let content_storage = Arc::new(FilesystemContentStorage::new(test_dir.to_str().unwrap())?);

        let mut tag_storage = MockTagStorageMock::new();

        let content1 = Content::new("Content with tag1".to_string());
        let content2 = Content::new("Content with tag1 and tag2".to_string());
        let content_id1 = content1.id.to_string();
        let content_id2 = content2.id.to_string();

        content_storage.store(&content1).await?;
        content_storage.store(&content2).await?;

        tag_storage
            .expect_find_by_tag()
            .with(eq("tag1"))
            .times(1)
            .returning(move |_| Ok(vec![content_id1.clone(), content_id2.clone()]));

        let tag1_content_ids = tag_storage.find_by_tag("tag1").await?;
        assert_eq!(tag1_content_ids.len(), 2);

        let mut retrieved_content = Vec::new();
        for id in tag1_content_ids {
            if let Some(content) = content_storage.get(&id).await? {
                retrieved_content.push(content);
            }
        }

        assert_eq!(retrieved_content.len(), 2);

        cleanup_test_dir(test_dir);

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_real_redis_integration() -> ClassifyResult<()> {
        let test_dir = setup_test_dir();

        let content_storage = Arc::new(FilesystemContentStorage::new(test_dir.to_str().unwrap())?);

        let tag_storage = Arc::new(RedisTagStorage::new("redis://localhost", None).await?);

        let content = Content::new("Real Redis integration test".to_string());
        let content_id = content.id.to_string();
        let tags = vec![
            "integration".to_string(),
            "test".to_string(),
            "redis".to_string(),
        ];

        content_storage.store(&content).await?;

        tag_storage.add_tags(&content_id, &tags).await?;

        let content_ids = tag_storage.find_by_tag("integration").await?;
        assert!(content_ids.contains(&content_id));

        let all_tags = tag_storage.list_tags().await?;
        assert!(all_tags.contains(&"integration".to_string()));
        assert!(all_tags.contains(&"test".to_string()));
        assert!(all_tags.contains(&"redis".to_string()));

        let retrieved = content_storage.get(&content_id).await?;
        assert!(retrieved.is_some());

        let retrieved_tags = tag_storage.get_tags(&content_id).await?;
        assert_eq!(retrieved_tags.len(), 3);

        content_storage.delete(&content_id).await?;
        tag_storage.remove_tags(&content_id, &tags).await?;
        cleanup_test_dir(test_dir);

        Ok(())
    }
}
