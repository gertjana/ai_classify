#[cfg(test)]
extern crate mockall;

pub mod api;
pub mod classifier;
pub mod config;
pub mod storage;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

/// Represents a piece of content with its classification tags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// Unique identifier for the content
    pub id: Uuid,
    /// Original text content or URL
    pub content: String,
    /// Classification tags assigned to the content
    pub tags: Vec<String>,
    /// When the content was created
    pub created_at: DateTime<Utc>,
    /// When the content was last updated
    pub updated_at: DateTime<Utc>,
}

impl Content {
    pub fn new(content: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            content,
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

/// Represents a classification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyRequest {
    /// The content to classify (text or URL)
    pub content: String,
}

/// Represents a classification response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyResponse {
    /// The classified content
    pub content: Content,
    /// Whether the classification was successful
    pub success: bool,
    /// Any error message
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

/// Result type for the application
pub type ClassifyResult<T> = Result<T, ClassifyError>;
