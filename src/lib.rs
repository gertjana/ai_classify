#[cfg(test)]
extern crate mockall;

pub mod api;
pub mod classifier;
pub mod config;
pub mod storage;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub id: Uuid,
    pub content: String,
    pub content_hash: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Content {
    pub fn new(content: String) -> Self {
        let now = Utc::now();
        let content_hash = Self::generate_hash(&content);

        Self {
            id: Uuid::new_v4(),
            content,
            content_hash: Some(content_hash),
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self.updated_at = Utc::now();
        self
    }

    /// Check if content is a URL
    pub fn is_url(&self) -> bool {
        self.content.starts_with("http://") || self.content.starts_with("https://")
    }

    /// Generate a SHA-256 hash of the content string
    pub fn generate_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

impl fmt::Display for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Content {{ id: {}, content: {}, tags: {:?} }}",
            self.id,
            if self.content.len() > 30 {
                format!("{}...", &self.content[..30])
            } else {
                self.content.clone()
            },
            self.tags
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyRequest {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyResponse {
    pub content: Content,
    pub success: bool,
    pub error: Option<String>,
}

/// Represents a content query response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentQueryResponse {
    /// The content items matching the query
    pub items: Vec<Content>,
    /// The tags that were queried
    pub tags: Vec<String>,
    /// Total number of items found
    pub count: usize,
    /// Whether the query was successful
    pub success: bool,
    /// Any error message
    pub error: Option<String>,
}

/// Represents a tags list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsResponse {
    /// List of all tags in the system
    pub tags: Vec<String>,
    /// Total number of tags
    pub count: usize,
    /// Whether the operation was successful
    pub success: bool,
    /// Any error message
    pub error: Option<String>,
}

/// Application error types
#[derive(Debug, Error)]
pub enum ClassifyError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Classification error: {0}")]
    ClassificationError(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("URL error: {0}")]
    UrlError(String),

    #[error("HTTP error: {0}")]
    HttpError(String),
}

pub type ClassifyResult<T> = Result<T, ClassifyError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_generation() {
        let text = "Test content for hashing";
        let content1 = Content::new(text.to_string());
        let content2 = Content::new(text.to_string());

        assert!(content1.content_hash.is_some());
        assert!(content2.content_hash.is_some());

        assert_eq!(content1.content_hash, content2.content_hash);

        let content3 = Content::new("Different content".to_string());
        assert_ne!(content1.content_hash, content3.content_hash);

        let direct_hash = Content::generate_hash(text);
        assert_eq!(Some(direct_hash), content1.content_hash);
    }
}
