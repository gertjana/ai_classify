pub mod claude;

#[cfg(test)]
mod claude_test;

use crate::ClassifyResult;
use async_trait::async_trait;
use std::sync::Arc;

/// Classifier trait for classifying content
#[async_trait]
pub trait Classifier: Send + Sync {
    /// Classify content and return tags
    async fn classify(&self, content: &str) -> ClassifyResult<Vec<String>>;

    /// Extract and classify content from a URL
    async fn classify_url(&self, url: &str) -> ClassifyResult<Vec<String>>;
}

/// Classifier factory
pub async fn create_classifier(
    classifier_type: &crate::config::ClassifierType,
    config: &crate::config::ClassifierConfig,
) -> ClassifyResult<Arc<dyn Classifier>> {
    match classifier_type {
        crate::config::ClassifierType::Claude => {
            let classifier = claude::ClaudeClassifier::new(
                config.anthropic_api_key.as_deref(),
                config.max_prompt_length,
            )?;
            Ok(Arc::new(classifier))
        } // Add more classifier types as needed
    }
}
