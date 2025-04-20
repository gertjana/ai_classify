use crate::classifier::Classifier;
use crate::classifier::claude::ClaudeClassifier;
use crate::ClassifyResult;

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a classifier with mocked HTTP client
    fn create_test_classifier() -> ClaudeClassifier {
        // Creating a classifier without API key to use fallback classification
        ClaudeClassifier::new(None, 10000).unwrap()
    }

    #[tokio::test]
    async fn test_classify_fallback() -> ClassifyResult<()> {
        // Create classifier with no API key to force fallback
        let classifier = create_test_classifier();

        // Test with Rust content
        let rust_content = "This is a test about Rust programming language";
        let tags = classifier.classify(rust_content).await?;

        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"programming".to_string()));

        // Test with web content
        let web_content = "This is about web development with HTML and HTTP";
        let tags = classifier.classify(web_content).await?;

        assert!(tags.contains(&"web".to_string()));

        // Test with database content
        let db_content = "Working with SQL databases and Redis";
        let tags = classifier.classify(db_content).await?;

        assert!(tags.contains(&"database".to_string()));

        // Test with AI content
        let ai_content = "Machine Learning and AI development";
        let tags = classifier.classify(ai_content).await?;

        assert!(tags.contains(&"ai".to_string()));

        // Test with unrelated content
        let unrelated_content = "Something completely unrelated to any keywords";
        let tags = classifier.classify(unrelated_content).await?;

        assert!(tags.contains(&"unclassified".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_content_truncation() -> ClassifyResult<()> {
        // Create classifier with small max length
        let classifier = ClaudeClassifier::new(None, 20).unwrap();

        // Long content that should be truncated
        let long_content = "This is a very long content that should be truncated according to the max length setting";
        let truncated = classifier.truncate_content(long_content);

        // Should be truncated to max length
        assert!(truncated.len() > 20); // Includes the truncation message
        assert!(truncated.starts_with("This is a very long"));
        assert!(truncated.contains("content truncated"));

        // Short content should not be truncated
        let short_content = "Short content";
        let not_truncated = classifier.truncate_content(short_content);

        assert_eq!(not_truncated, short_content);

        Ok(())
    }

    // This test would normally require mocking the API call, but we can test the
    // URL validation part of classify_url without making real API calls
    #[tokio::test]
    async fn test_classify_url_validation() -> ClassifyResult<()> {
        // Create classifier with no API key to force fallback
        let classifier = create_test_classifier();

        // Invalid URL should return error
        let invalid_url = "not-a-url";
        let result = classifier.classify_url(invalid_url).await;

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(format!("{:?}", e).contains("Invalid URL"));
        }

        Ok(())
    }
}
