use std::process::exit;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

use classify::api::{start_server, AppState};
use classify::classifier::create_classifier;
use classify::config::AppConfig;
use classify::storage::{create_content_storage, create_tag_storage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    info!("Starting classify application...");

    // Initialize configuration
    let config = match AppConfig::init() {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to initialize configuration: {}", e);
            exit(1);
        }
    };

    // Create content storage
    let content_storage =
        match create_content_storage(&config.storage.storage_type, &config.storage).await {
            Ok(storage) => storage,
            Err(e) => {
                error!("Failed to initialize content storage: {}", e);
                exit(1);
            }
        };

    info!(
        "Content storage initialized: {:?}",
        config.storage.storage_type
    );

    // Create tag storage
    let tag_storage =
        match create_tag_storage(&config.tag_storage.tag_storage_type, &config.tag_storage).await {
            Ok(storage) => storage,
            Err(e) => {
                error!("Failed to initialize tag storage: {}", e);
                exit(1);
            }
        };

    info!(
        "Tag storage initialized: {:?}",
        config.tag_storage.tag_storage_type
    );

    // Create classifier
    let classifier =
        match create_classifier(&config.classifier.classifier_type, &config.classifier).await {
            Ok(classifier) => classifier,
            Err(e) => {
                error!("Failed to initialize classifier: {}", e);
                exit(1);
            }
        };

    info!(
        "Classifier initialized: {:?}",
        config.classifier.classifier_type
    );

    // Create app state
    let app_state = AppState::new(classifier, content_storage, tag_storage);

    // Get API address
    let addr = match config.api_addr() {
        Ok(addr) => addr,
        Err(e) => {
            error!("Failed to get API address: {}", e);
            exit(1);
        }
    };

    // Start API server
    info!(
        "Starting API server on {}:{}",
        config.api.host, config.api.port
    );

    if let Err(e) = start_server(app_state, addr).await {
        error!("Server error: {}", e);
        exit(1);
    }

    Ok(())
}
