use crate::classifier::chatgpt::ChatGptClassifier;
use crate::classifier::Classifier;
use crate::ClassifyResult;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_classifier() -> ChatGptClassifier {
        ChatGptClassifier::new(None, 10000).unwrap()
    }

    #[tokio::test]
    async fn test_content_truncation() -> ClassifyResult<()> {
        let classifier = ChatGptClassifier::new(None, 20).unwrap();

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
