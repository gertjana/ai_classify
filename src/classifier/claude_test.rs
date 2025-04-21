use crate::classifier::claude::ClaudeClassifier;
use crate::classifier::Classifier;
use crate::ClassifyResult;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_classifier() -> ClaudeClassifier {
        ClaudeClassifier::new(None, 10000).unwrap()
    }

    #[tokio::test]
    async fn test_classify_fallback() -> ClassifyResult<()> {
        let classifier = create_test_classifier();

        let rust_content = "This is a test about Rust programming language";
        let tags = classifier.classify(rust_content).await?;

        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"programming".to_string()));

        let web_content = "This is about web development with HTML and HTTP";
        let tags = classifier.classify(web_content).await?;

        assert!(tags.contains(&"web".to_string()));

        let db_content = "Working with SQL databases and Redis";
        let tags = classifier.classify(db_content).await?;

        assert!(tags.contains(&"database".to_string()));

        let ai_content = "Machine Learning and AI development";
        let tags = classifier.classify(ai_content).await?;

        assert!(tags.contains(&"ai".to_string()));

        let unrelated_content = "Something completely unrelated to any keywords";
        let tags = classifier.classify(unrelated_content).await?;

        assert!(tags.contains(&"unclassified".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_content_truncation() -> ClassifyResult<()> {
        let classifier = ClaudeClassifier::new(None, 20).unwrap();

        let long_content = "This is a very long content that should be truncated according to the max length setting";
        let truncated = classifier.truncate_content(long_content);

        assert!(truncated.len() > 20); // Includes the truncation message
        assert!(truncated.starts_with("This is a very long"));
        assert!(truncated.contains("content truncated"));

        let short_content = "Short content";
        let not_truncated = classifier.truncate_content(short_content);

        assert_eq!(not_truncated, short_content);

        Ok(())
    }

    #[tokio::test]
    async fn test_classify_url_validation() -> ClassifyResult<()> {
        let classifier = create_test_classifier();

        let invalid_url = "not-a-url";
        let result = classifier.classify_url(invalid_url).await;

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(format!("{:?}", e).contains("Invalid URL"));
        }

        Ok(())
    }
}
