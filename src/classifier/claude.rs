use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{ClassifyError, ClassifyResult};
use crate::classifier::Classifier;

const MAX_TAGS: usize = 5;
const CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";

/// Claude AI-based classifier
pub struct ClaudeClassifier {
    /// Anthropic API key
    api_key: Option<String>,
    /// HTTP client
    client: reqwest::Client,
    /// Maximum prompt length in characters
    max_prompt_length: usize,
}

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    system: String,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<Content>,
}

#[derive(Debug, Deserialize)]
struct Content {
    text: String,
    #[serde(rename = "type")]
    content_type: String,
}

impl ClaudeClassifier {
    /// Create a new Claude classifier
    pub fn new(api_key: Option<&str>, max_prompt_length: usize) -> ClassifyResult<Self> {
        Ok(Self {
            api_key: api_key.map(String::from),
            client: reqwest::Client::new(),
            max_prompt_length,
        })
    }

    /// Truncate content to maximum length
    fn truncate_content(&self, content: &str) -> String {
        if content.len() <= self.max_prompt_length {
            content.to_string()
        } else {
            let truncated = &content[0..self.max_prompt_length];
            format!("{}... [content truncated, original length: {}]", truncated, content.len())
        }
    }

    /// Extract content from a URL
    async fn extract_content_from_url(&self, url: &str) -> ClassifyResult<String> {
        // Validate URL
        let url = Url::parse(url)
            .map_err(|e| ClassifyError::UrlError(format!("Invalid URL: {}", e)))?;

        // Fetch URL content
        let response = self.client.get(url.as_str())
            .send()
            .await
            .map_err(|e| ClassifyError::HttpError(format!("Failed to fetch URL: {}", e)))?;

        if !response.status().is_success() {
            return Err(ClassifyError::HttpError(format!(
                "Failed to fetch URL: HTTP status {}",
                response.status()
            )));
        }

        // Get text content
        let content = response.text().await
            .map_err(|e| ClassifyError::HttpError(format!("Failed to read response body: {}", e)))?;

        // Truncate content if needed
        Ok(self.truncate_content(&content))
    }

    /// Call Claude API to classify content
    async fn call_claude_api(&self, content: &str) -> ClassifyResult<Vec<String>> {
        // Check if API key is available
        let api_key = match &self.api_key {
            Some(key) => key,
            None => return self.fallback_classification(content).await,
        };

        // Set up headers
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&format!("{}", api_key))
                .map_err(|e| ClassifyError::ClassificationError(format!("Invalid API key: {}", e)))?
        );

        // Truncate content if needed
        let truncated_content = self.truncate_content(content);

        // Prepare the prompt
        let system_prompt = format!(
            "You are a helpful content tagger that analyzes text and extracts relevant tags. \
            Provide exactly up to {} descriptive tags that categorize the content. \
            Return ONLY the tags separated by commas, nothing else. \
            Tags should be single words or short phrases.",
            MAX_TAGS
        );

        let user_prompt = format!(
            "Please analyze the following content and provide up to {} descriptive tags: \n\n{}",
            MAX_TAGS,
            truncated_content
        );

        // Create the request payload
        let request = ClaudeRequest {
            model: "claude-3-haiku-20240307".to_string(),
            max_tokens: 100,
            messages: vec![Message {
                role: "user".to_string(),
                content: user_prompt,
            }],
            system: system_prompt,
        };

        // Make the API call
        let response = self.client.post(CLAUDE_API_URL)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|e| ClassifyError::ClassificationError(format!("Failed to call Claude API: {}", e)))?;

        let status = response.status();

        // Check if the response was successful
        if !status.is_success() {
            let error_text = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(ClassifyError::ClassificationError(format!(
                "Claude API error: HTTP status {}, {}",
                status,
                error_text
            )));
        }

        // Parse the response
        let claude_response = response.json::<ClaudeResponse>().await
            .map_err(|e| ClassifyError::ClassificationError(format!("Failed to parse Claude response: {}", e)))?;

        // Extract tags from the response
        let tags_text = claude_response.content
            .iter()
            .filter(|content| content.content_type == "text")
            .map(|content| content.text.clone())
            .collect::<Vec<_>>()
            .join("");

        // Split tags by comma and clean them up
        let tags = tags_text
            .split(',')
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .take(MAX_TAGS)
            .collect();

        Ok(tags)
    }

    /// Fallback classification when API key is not available
    async fn fallback_classification(&self, content: &str) -> ClassifyResult<Vec<String>> {
        // Simple keyword-based classification
        let content = content.to_lowercase();
        let mut tags = Vec::new();

        // Sample tags based on content keywords
        if content.contains("rust") {
            tags.push("programming".to_string());
            tags.push("rust".to_string());
        }

        if content.contains("web") || content.contains("http") || content.contains("html") {
            tags.push("web".to_string());
        }

        if content.contains("api") || content.contains("rest") || content.contains("graphql") {
            tags.push("api".to_string());
        }

        if content.contains("database") || content.contains("sql") || content.contains("redis") {
            tags.push("database".to_string());
        }

        if content.contains("ai") || content.contains("machine learning") || content.contains("ml") {
            tags.push("ai".to_string());
        }

        // If no tags were found, add a default tag
        if tags.is_empty() {
            tags.push("unclassified".to_string());
        }

        // Limit to MAX_TAGS
        tags.truncate(MAX_TAGS);

        Ok(tags)
    }
}

#[async_trait]
impl Classifier for ClaudeClassifier {
    async fn classify(&self, content: &str) -> ClassifyResult<Vec<String>> {
        self.call_claude_api(content).await
    }

    async fn classify_url(&self, url: &str) -> ClassifyResult<Vec<String>> {
        // Extract content from URL
        let content = self.extract_content_from_url(url).await?;

        // Classify the extracted content
        self.classify(&content).await
    }
}
