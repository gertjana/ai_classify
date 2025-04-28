use crate::ClassifyError;
use serde::Deserialize;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::OnceLock;
use uuid;

static CONFIG: OnceLock<AppConfig> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub api: ApiConfig,
    pub storage: StorageConfig,
    pub tag_storage: TagStorageConfig,
    pub classifier: ClassifierConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub storage_type: StorageType,
    pub content_storage_path: String,
    pub s3_bucket: Option<String>,
    pub s3_prefix: Option<String>,
    pub s3_region: Option<String>,
    pub s3_profile: Option<String>,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TagStorageConfig {
    pub tag_storage_type: TagStorageType,
    pub redis_url: String,
    pub redis_password: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClassifierConfig {
    pub classifier_type: ClassifierType,
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub openai_model: Option<String>,
    pub max_prompt_length: usize,
}

/// Storage types
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    Filesystem,
    S3,
}

/// Tag storage types
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TagStorageType {
    Redis,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ClassifierType {
    Claude,
    ChatGpt,
}

impl AppConfig {
    pub fn init() -> Result<&'static Self, ClassifyError> {
        dotenvy::dotenv().ok();

        let api_host = std::env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let api_port = std::env::var("API_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .map_err(|e| ClassifyError::ConfigError(format!("Invalid API_PORT: {}", e)))?;

        let api_key = std::env::var("API_KEY").unwrap_or_else(|_| {
            let random_key = uuid::Uuid::new_v4().to_string();
            eprintln!(
                "No API_KEY found in environment, generated random key: {}",
                random_key
            );
            random_key
        });

        let storage_type = std::env::var("STORAGE_TYPE")
            .unwrap_or_else(|_| "filesystem".to_string())
            .parse()
            .map_err(|e| ClassifyError::ConfigError(format!("Invalid STORAGE_TYPE: {}", e)))?;

        let content_storage_path =
            std::env::var("CONTENT_STORAGE_PATH").unwrap_or_else(|_| "./data/content".to_string());

        // S3 configuration
        let s3_bucket = std::env::var("S3_BUCKET").ok();
        let s3_prefix = std::env::var("S3_PREFIX").ok();
        let s3_region = std::env::var("S3_REGION").ok();
        let s3_profile = std::env::var("AWS_PROFILE").ok();
        let s3_access_key = std::env::var("AWS_ACCESS_KEY_ID").ok();
        let s3_secret_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok();

        let tag_storage_type = std::env::var("TAG_STORAGE_TYPE")
            .unwrap_or_else(|_| "redis".to_string())
            .parse()
            .map_err(|e| ClassifyError::ConfigError(format!("Invalid TAG_STORAGE_TYPE: {}", e)))?;

        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let redis_password = std::env::var("REDIS_PASSWORD").ok();

        let classifier_type = std::env::var("CLASSIFIER_TYPE")
            .unwrap_or_else(|_| "claude".to_string())
            .parse()
            .map_err(|e| ClassifyError::ConfigError(format!("Invalid CLASSIFIER_TYPE: {}", e)))?;

        let anthropic_api_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let openai_api_key = std::env::var("OPENAI_API_KEY").ok();
        let openai_model = std::env::var("OPENAI_MODEL").ok();

        let max_prompt_length = std::env::var("MAX_PROMPT_LENGTH")
            .unwrap_or_else(|_| "200000".to_string())
            .parse::<usize>()
            .map_err(|e| ClassifyError::ConfigError(format!("Invalid MAX_PROMPT_LENGTH: {}", e)))?;

        let config = AppConfig {
            api: ApiConfig {
                host: api_host,
                port: api_port,
                api_key,
            },
            storage: StorageConfig {
                storage_type,
                content_storage_path,
                s3_bucket,
                s3_prefix,
                s3_region,
                s3_profile,
                s3_access_key,
                s3_secret_key,
            },
            tag_storage: TagStorageConfig {
                tag_storage_type,
                redis_url,
                redis_password,
            },
            classifier: ClassifierConfig {
                classifier_type,
                anthropic_api_key,
                openai_api_key,
                openai_model,
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
            "s3" => Ok(StorageType::S3),
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
            "chatgpt" => Ok(ClassifierType::ChatGpt),
            _ => Err(format!("Unknown classifier type: {}", s)),
        }
    }
}
