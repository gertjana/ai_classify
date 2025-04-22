use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::Response,
};
use std::sync::Arc;
use tracing::warn;

use crate::api::AppState;
use crate::config::AppConfig;

pub async fn validate_api_key(
    State(_state): State<Arc<AppState>>,
    req: Request<Body>,
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    let config = AppConfig::get().map_err(|_| {
        warn!("Failed to get application configuration");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let expected_api_key = &config.api.api_key;

    let api_key = req
        .headers()
        .get("X-Api-Key")
        .and_then(|value| value.to_str().ok());

    match api_key {
        Some(key) if key == expected_api_key => Ok(next.run(req).await),
        _ => {
            warn!("Invalid or missing API key");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
