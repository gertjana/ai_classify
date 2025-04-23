use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::classifier::Classifier;
use crate::{ClassifyError, ClassifyResult};

const MAX_TAGS: usize = 5;
const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

pub struct ChatGptClassifier {
    api_key: Option<String>,
    model: String,
    client: reqwest::Client,
    max_prompt_length: usize,
}

#[derive(Debug, Serialize)]
struct ChatGptRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatGptResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

impl ChatGptClassifier {
    /// Create a new ChatGPT classifier
    pub fn new(api_key: Option<&str>, max_prompt_length: usize) -> ClassifyResult<Self> {
        Ok(Self {
            api_key: api_key.map(String::from),
            model: "gpt-4o-mini".to_string(), // Use GPT-4o-mini by default
            client: reqwest::Client::new(),
            max_prompt_length,
        })
    }

    /// Create a new ChatGPT classifier with a specific model
    pub fn with_model(
        api_key: Option<&str>,
        model: &str,
        max_prompt_length: usize,
    ) -> ClassifyResult<Self> {
        let mut classifier = Self::new(api_key, max_prompt_length)?;
        classifier.model = model.to_string();
        Ok(classifier)
    }

    /// Truncate content to maximum length
    pub fn truncate_content(&self, content: &str) -> String {
        if content.len() <= self.max_prompt_length {
            content.to_string()
        } else {
            let truncated = &content[0..self.max_prompt_length];
            format!(
                "{}... [content truncated, original length: {}]",
                truncated,
                content.len()
            )
        }
    }

    /// Extract content from a URL
    async fn extract_content_from_url(&self, url: &str) -> ClassifyResult<String> {
        let url =
            Url::parse(url).map_err(|e| ClassifyError::UrlError(format!("Invalid URL: {}", e)))?;

        let response = self
            .client
            .get(url.as_str())
            .send()
            .await
            .map_err(|e| ClassifyError::HttpError(format!("Failed to fetch URL: {}", e)))?;

        if !response.status().is_success() {
            return Err(ClassifyError::HttpError(format!(
                "Failed to fetch URL: HTTP status {}",
                response.status()
            )));
        }

        let content = response.text().await.map_err(|e| {
            ClassifyError::HttpError(format!("Failed to read response body: {}", e))
        })?;

        Ok(self.truncate_content(&content))
    }

    async fn call_chatgpt_api(&self, content: &str) -> ClassifyResult<Vec<String>> {
        let api_key = match &self.api_key {
            Some(key) => key,
            None => return self.fallback_classification(content).await,
        };

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key)).map_err(|e| {
                ClassifyError::ClassificationError(format!("Invalid API key: {}", e))
            })?,
        );

        let truncated_content = self.truncate_content(content);

        let system_prompt = format!(
            "You are a helpful content tagger that analyzes text and extracts relevant tags. \
            Provide exactly up to {} descriptive tags that categorize the content. \
            Return ONLY the tags separated by commas, nothing else. \
            Tags should be single words or short phrases.",
            MAX_TAGS
        );

        let user_prompt = format!(
            "Please analyze the following content and provide up to {} descriptive tags: \n\n{}",
            MAX_TAGS, truncated_content
        );

        let request = ChatGptRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                Message {
                    role: "user".to_string(),
                    content: user_prompt,
                },
            ],
            temperature: 0.3,
            max_tokens: 100,
        };

        let response = self
            .client
            .post(OPENAI_API_URL)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                ClassifyError::ClassificationError(format!("Failed to call OpenAI API: {}", e))
            })?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(ClassifyError::ClassificationError(format!(
                "OpenAI API error: HTTP status {}, {}",
                status, error_text
            )));
        }

        let chatgpt_response = response.json::<ChatGptResponse>().await.map_err(|e| {
            ClassifyError::ClassificationError(format!("Failed to parse OpenAI response: {}", e))
        })?;

        if chatgpt_response.choices.is_empty() {
            return Err(ClassifyError::ClassificationError(
                "Empty response from OpenAI API".to_string(),
            ));
        }

        let tags_text = chatgpt_response.choices[0].message.content.clone();

        let tags = tags_text
            .split(',')
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .take(MAX_TAGS)
            .collect();

        Ok(tags)
    }

    async fn fallback_classification(&self, _content: &str) -> ClassifyResult<Vec<String>> {
        Err(ClassifyError::ClassificationError(
            "OpenAI API key is required for classification".to_string(),
        ))
    }
}

#[async_trait]
impl Classifier for ChatGptClassifier {
    async fn classify(&self, content: &str) -> ClassifyResult<Vec<String>> {
        self.call_chatgpt_api(content).await
    }

    async fn classify_url(&self, url: &str) -> ClassifyResult<Vec<String>> {
        let content = self.extract_content_from_url(url).await?;
        self.classify(&content).await
    }
}
