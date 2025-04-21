use mockall::mock;
use mockall::predicate::*;

use crate::storage::TagStorage;
use crate::ClassifyResult;

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

        mock.add_tags(content_id, &tags).await?;

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

        mock.expect_get_tags()
            .with(eq(content_id))
            .times(1)
            .returning(|_| Ok(vec!["testing".to_string(), "rust".to_string()]));

        let content_tags = mock.get_tags(content_id).await?;

        assert_eq!(content_tags.len(), 2);
        assert!(content_tags.contains(&"testing".to_string()));
        assert!(content_tags.contains(&"rust".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_list_tags() -> ClassifyResult<()> {
        let mut mock = MockTagStorageMock::new();

        mock.expect_list_tags().times(1).returning(|| {
            Ok(vec![
                "rust".to_string(),
                "programming".to_string(),
                "testing".to_string(),
            ])
        });

        let all_tags = mock.list_tags().await?;

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

        mock.expect_find_by_tag()
            .with(eq(tag))
            .times(1)
            .returning(|_| Ok(vec!["content-1".to_string(), "content-2".to_string()]));

        let contents = mock.find_by_tag(tag).await?;

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

        mock.remove_tags(content_id, &tags_to_remove).await?;

        let content_tags = mock.get_tags(content_id).await?;
        assert_eq!(content_tags.len(), 2);
        assert!(content_tags.contains(&"rust".to_string()));
        assert!(content_tags.contains(&"testing".to_string()));
        assert!(!content_tags.contains(&"programming".to_string()));

        Ok(())
    }
}
