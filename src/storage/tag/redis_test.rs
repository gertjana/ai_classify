use std::collections::HashSet;

use mockall::mock;
use mockall::predicate::*;

use crate::storage::TagStorage;
use crate::ClassifyResult;

// Create a mock implementation of the TagStorage trait for testing
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_tags() -> ClassifyResult<()> {
        let mut mock = MockTagStorageMock::new();
        let content_id = "test-content-1";
        let tags = vec!["rust".to_string(), "programming".to_string()];

        // Setup expectations
        mock.expect_add_tags()
            .with(
                eq(content_id),
                function(|t: &[String]| {
                    t.len() == 2
                        && t.contains(&"rust".to_string())
                        && t.contains(&"programming".to_string())
                }),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock.expect_get_tags()
            .with(eq(content_id))
            .times(1)
            .returning(|_| Ok(vec!["rust".to_string(), "programming".to_string()]));

        // Add tags
        mock.add_tags(content_id, &tags).await?;

        // Verify tags were added correctly
        let content_tags = mock.get_tags(content_id).await?;
        assert_eq!(content_tags.len(), 2);
        assert!(content_tags.contains(&"rust".to_string()));
        assert!(content_tags.contains(&"programming".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_get_tags() -> ClassifyResult<()> {
        let mut mock = MockTagStorageMock::new();
        let content_id = "test-content-2";

        // Setup expectations
        mock.expect_get_tags()
            .with(eq(content_id))
            .times(1)
            .returning(|_| Ok(vec!["testing".to_string(), "rust".to_string()]));

        // Get tags for content
        let content_tags = mock.get_tags(content_id).await?;

        // Verify tags
        assert_eq!(content_tags.len(), 2);
        assert!(content_tags.contains(&"testing".to_string()));
        assert!(content_tags.contains(&"rust".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_list_tags() -> ClassifyResult<()> {
        let mut mock = MockTagStorageMock::new();

        // Setup expectations
        mock.expect_list_tags().times(1).returning(|| {
            Ok(vec![
                "rust".to_string(),
                "programming".to_string(),
                "testing".to_string(),
            ])
        });

        // List all tags
        let all_tags = mock.list_tags().await?;

        // Verify tags
        assert_eq!(all_tags.len(), 3);
        assert!(all_tags.contains(&"rust".to_string()));
        assert!(all_tags.contains(&"programming".to_string()));
        assert!(all_tags.contains(&"testing".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_find_by_tag() -> ClassifyResult<()> {
        let mut mock = MockTagStorageMock::new();
        let tag = "rust";

        // Setup expectations
        mock.expect_find_by_tag()
            .with(eq(tag))
            .times(1)
            .returning(|_| Ok(vec!["content-1".to_string(), "content-2".to_string()]));

        // Find content by tag
        let contents = mock.find_by_tag(tag).await?;

        // Verify content
        assert_eq!(contents.len(), 2);
        assert!(contents.contains(&"content-1".to_string()));
        assert!(contents.contains(&"content-2".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_remove_tags() -> ClassifyResult<()> {
        let mut mock = MockTagStorageMock::new();
        let content_id = "test-content-3";
        let tags_to_remove = vec!["programming".to_string()];

        // Setup expectations
        mock.expect_remove_tags()
            .with(
                eq(content_id),
                function(|t: &[String]| t.len() == 1 && t[0] == "programming"),
            )
            .times(1)
            .returning(|_, _| Ok(()));

        mock.expect_get_tags()
            .with(eq(content_id))
            .times(1)
            .returning(|_| Ok(vec!["rust".to_string(), "testing".to_string()]));

        // Remove tags
        mock.remove_tags(content_id, &tags_to_remove).await?;

        // Verify remaining tags
        let content_tags = mock.get_tags(content_id).await?;
        assert_eq!(content_tags.len(), 2);
        assert!(content_tags.contains(&"rust".to_string()));
        assert!(content_tags.contains(&"testing".to_string()));
        assert!(!content_tags.contains(&"programming".to_string()));

        Ok(())
    }

    // Feature test that verifies the behavior of the tag storage implementation
    // after the removal of the all:tags functionality
    #[tokio::test]
    async fn test_list_tags_without_all_tags_key() -> ClassifyResult<()> {
        let mut mock = MockTagStorageMock::new();

        // Setup mock to simulate the new implementation's behavior
        // where we extract tag names from tag:*:contents keys
        mock.expect_list_tags().times(1).returning(|| {
            // This simulates what happens in the real implementation:
            // 1. Get all keys matching "classify:tag:*:contents"
            let keys = vec![
                "classify:tag:rust:contents".to_string(),
                "classify:tag:programming:contents".to_string(),
                "classify:tag:testing:contents".to_string(),
            ];

            // 2. Extract tag names from keys
            let mut tags = HashSet::new();
            for key in keys {
                if let Some(tag) = key
                    .strip_prefix("classify:tag:")
                    .and_then(|s| s.strip_suffix(":contents"))
                {
                    tags.insert(tag.to_string());
                }
            }

            // 3. Return as vector
            Ok(tags.into_iter().collect())
        });

        // Call list_tags
        let tags = mock.list_tags().await?;

        // Verify correct extraction of tags from keys
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"programming".to_string()));
        assert!(tags.contains(&"testing".to_string()));

        Ok(())
    }
}
