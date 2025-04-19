use serde::Deserialize;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::OnceLock;
use crate::ClassifyError;

static CONFIG: OnceLock<AppConfig> = OnceLock::new();

/// Application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    /// API configuration
    pub api: ApiConfig,
    /// Storage configuration
    pub storage: StorageConfig,
    /// Tag storage configuration
    pub tag_storage: TagStorageConfig,
    /// Classifier configuration
    pub classifier: ClassifierConfig,
}

/// API configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    /// Host to bind to
    pub host: String,
    /// Port to bind to
    pub port: u16,
}

/// Storage configuration
#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    /// Storage type
    pub storage_type: StorageType,
    /// Path to content storage (for filesystem)
    pub content_storage_path: String,
}

/// Tag storage configuration
#[derive(Debug, Clone, Deserialize)]
pub struct TagStorageConfig {
    /// Tag storage type
    pub tag_storage_type: TagStorageType,
    /// Redis URL (for Redis)
    pub redis_url: String,
    /// Redis password (for Redis)
    pub redis_password: Option<String>,
}

/// Classifier configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ClassifierConfig {
    /// Classifier type
    pub classifier_type: ClassifierType,
    /// Anthropic API key (for Claude)
    pub anthropic_api_key: Option<String>,
    /// Maximum prompt length in characters
    pub max_prompt_length: usize,
}

/// Storage types
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    /// Filesystem storage
    Filesystem,
    // Add more storage types as needed
}

/// Tag storage types
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TagStorageType {
    /// Redis storage
    Redis,
    // Add more tag storage types as needed
}

/// Classifier types
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ClassifierType {
    /// Claude classifier
    Claude,
    // Add more classifier types as needed
}

impl AppConfig {
    /// Initialize the application configuration
    pub fn init() -> Result<&'static Self, ClassifyError> {
        // Load .env file if it exists
        dotenvy::dotenv().ok();

        let api_host = std::env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let api_port = std::env::var("API_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .map_err(|e| ClassifyError::ConfigError(format!("Invalid API_PORT: {}", e)))?;

        let storage_type = std::env::var("STORAGE_TYPE")
            .unwrap_or_else(|_| "filesystem".to_string())
            .parse()
            .map_err(|e| ClassifyError::ConfigError(format!("Invalid STORAGE_TYPE: {}", e)))?;

        let content_storage_path = std::env::var("CONTENT_STORAGE_PATH")
            .unwrap_or_else(|_| "./data/content".to_string());

        let tag_storage_type = std::env::var("TAG_STORAGE_TYPE")
            .unwrap_or_else(|_| "redis".to_string())
            .parse()
            .map_err(|e| ClassifyError::ConfigError(format!("Invalid TAG_STORAGE_TYPE: {}", e)))?;

        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let redis_password = std::env::var("REDIS_PASSWORD").ok();

        let classifier_type = std::env::var("CLASSIFIER_TYPE")
            .unwrap_or_else(|_| "claude".to_string())
            .parse()
            .map_err(|e| ClassifyError::ConfigError(format!("Invalid CLASSIFIER_TYPE: {}", e)))?;

        let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY").ok();

        let max_prompt_length = std::env::var("MAX_PROMPT_LENGTH")
            .unwrap_or_else(|_| "200000".to_string())
            .parse::<usize>()
            .map_err(|e| ClassifyError::ConfigError(format!("Invalid MAX_PROMPT_LENGTH: {}", e)))?;

        let config = AppConfig {
            api: ApiConfig {
                host: api_host,
                port: api_port,
            },
            storage: StorageConfig {
                storage_type,
                content_storage_path,
            },
            tag_storage: TagStorageConfig {
                tag_storage_type,
                redis_url,
                redis_password,
            },
            classifier: ClassifierConfig {
                classifier_type,
                anthropic_api_key,
                max_prompt_length,
            },
        };

        CONFIG.get_or_init(|| config);
        Ok(CONFIG.get().unwrap())
    }

    /// Get the application configuration
    pub fn get() -> Result<&'static Self, ClassifyError> {
        CONFIG
            .get()
            .ok_or_else(|| ClassifyError::ConfigError("Configuration not initialized".to_string()))
    }

    /// Get socket address for API server
    pub fn api_addr(&self) -> Result<SocketAddr, ClassifyError> {
        let ip = IpAddr::from_str(&self.api.host)
            .map_err(|e| ClassifyError::ConfigError(format!("Invalid API host: {}", e)))?;
        Ok(SocketAddr::new(ip, self.api.port))
    }
}

impl FromStr for StorageType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "filesystem" => Ok(StorageType::Filesystem),
            _ => Err(format!("Unknown storage type: {}", s)),
        }
    }
}

impl FromStr for TagStorageType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "redis" => Ok(TagStorageType::Redis),
            _ => Err(format!("Unknown tag storage type: {}", s)),
        }
    }
}

impl FromStr for ClassifierType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" => Ok(ClassifierType::Claude),
            _ => Err(format!("Unknown classifier type: {}", s)),
        }
    }
}
