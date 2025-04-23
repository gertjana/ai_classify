use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::classifier::Classifier;
use crate::storage::{ContentStorage, TagStorage};
use crate::{
    ClassifyError, ClassifyRequest, ClassifyResponse, Content, ContentQueryResponse, TagsResponse,
};

mod middleware;
#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct AppState {
    pub classifier: Arc<dyn Classifier>,
    pub content_storage: Arc<dyn ContentStorage>,
    pub tag_storage: Arc<dyn TagStorage>,
}

impl AppState {
    pub fn new(
        classifier: Arc<dyn Classifier>,
        content_storage: Arc<dyn ContentStorage>,
        tag_storage: Arc<dyn TagStorage>,
    ) -> Self {
        Self {
            classifier,
            content_storage,
            tag_storage,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub tags: String,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub id: Option<String>,
    pub removed_tags: Vec<String>,
    pub error: Option<String>,
}

pub fn create_router(state: AppState) -> Router {
    let shared_state = Arc::new(state);

    // Create a router for protected routes (requires API key)
    let protected_routes = Router::new()
        .route("/classify", post(classify_content))
        .route("/query", get(query_content))
        .route("/content/:id", delete(delete_content))
        .route("/content/:id", get(get_content_text))
        .route("/tags", get(get_tags))
        .layer(from_fn_with_state(
            shared_state.clone(),
            middleware::validate_api_key,
        ));

    // Create the main router, with the health check route unprotected
    Router::new()
        .route("/", get(health_check))
        .merge(protected_routes)
        .with_state(shared_state)
}

pub async fn start_server(app_state: AppState, addr: SocketAddr) -> Result<(), ClassifyError> {
    let app = create_router(app_state);

    info!("Starting server on {}", addr);

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| ClassifyError::ApiError(format!("Failed to bind: {}", e)))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| ClassifyError::ApiError(format!("Server error: {}", e)))
}

/// Health check endpoint
async fn health_check() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .body(axum::body::Body::empty())
        .unwrap()
}

/// Classify content endpoint
async fn classify_content(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ClassifyRequest>,
) -> Result<Json<ClassifyResponse>, ApiError> {
    info!("Received classification request");

    let content_hash = Content::generate_hash(&request.content);

    if let Some(existing_content) = state.content_storage.find_by_hash(&content_hash).await? {
        info!("Found existing content with the same hash");

        let response = ClassifyResponse {
            content: existing_content,
            success: true,
            error: None,
        };

        return Err(ApiError::Conflict(response));
    }

    let content = Content::new(request.content.clone());

    let tags = if content.is_url() {
        info!("Detected URL: {}", &content.content);
        state.classifier.classify_url(&content.content).await?
    } else {
        info!("Detected text content");
        state.classifier.classify(&content.content).await?
    };

    let content = content.with_tags(tags.clone());

    // RESEARCH: should the next two lines be in a transaction?
    state.content_storage.store(&content).await?;

    state
        .tag_storage
        .add_tags(&content.id.to_string(), &tags)
        .await?;

    let response = ClassifyResponse {
        content,
        success: true,
        error: None,
    };

    Ok(Json(response))
}

async fn query_content(
    State(state): State<Arc<AppState>>,
    Query(params): Query<QueryParams>,
) -> Result<Json<ContentQueryResponse>, ApiError> {
    info!("Received content query request for tags: {}", params.tags);

    // Parse tags from query string
    let tags: Vec<String> = params
        .tags
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if tags.is_empty() {
        return Err(ApiError::BadRequest("No valid tags provided".to_string()));
    }

    let mut content_ids = HashSet::new();
    for tag in &tags {
        let tag_content_ids = state.tag_storage.find_by_tag(tag).await?;
        for id in tag_content_ids {
            content_ids.insert(id);
        }
    }

    info!(
        "Found {} content items matching the tags",
        content_ids.len()
    );

    let mut items = Vec::new();
    for content_id in content_ids {
        if let Some(content) = state.content_storage.get(&content_id).await? {
            items.push(content);
        }
    }

    info!("Retrieved {} content items", items.len());

    items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    let count = items.len();

    let response = ContentQueryResponse {
        items,
        tags,
        count,
        success: true,
        error: None,
    };

    Ok(Json(response))
}

async fn delete_content(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DeleteResponse>, ApiError> {
    info!("Received delete content request for ID: {}", id);

    if (state.content_storage.get(&id).await?).is_some() {
        let tags = state.tag_storage.get_tags(&id).await?;
        info!("Content has {} tags that may need cleanup", tags.len());

        // RESEARCH: should deletion of content and tags be transactional?

        let deleted = state.content_storage.delete(&id).await?;

        if !deleted {
            return Err(ApiError::BadRequest(format!(
                "Failed to delete content with ID: {}",
                id
            )));
        }

        let mut orphaned_tags = Vec::new();

        for tag in &tags {
            let content_with_tag = state.tag_storage.find_by_tag(tag).await?;

            if content_with_tag.is_empty() {
                info!("Tag '{}' is now orphaned, will be removed", tag);
                orphaned_tags.push(tag.clone());
            }
        }

        state.tag_storage.remove_tags(&id, &tags).await?;

        let response = DeleteResponse {
            success: true,
            id: Some(id),
            removed_tags: orphaned_tags,
            error: None,
        };

        Ok(Json(response))
    } else {
        Err(ApiError::BadRequest(format!(
            "Content with ID {} not found",
            id
        )))
    }
}

async fn get_tags(State(state): State<Arc<AppState>>) -> Result<Json<TagsResponse>, ApiError> {
    info!("Received request for all tags");

    // Retrieve all tags from storage
    let tags = state.tag_storage.list_tags().await?;
    let count = tags.len();

    info!("Retrieved {} tags", count);

    // Return response
    let response = TagsResponse {
        tags,
        count,
        success: true,
        error: None,
    };

    Ok(Json(response))
}

/// Get content by ID endpoint (returns plain text)
async fn get_content_text(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    info!("Received get content text request for ID: {}", id);

    // Retrieve content from storage
    let content_option = state.content_storage.get(&id).await?;

    if let Some(content) = content_option {
        // Return the content text with 200 OK status and Content-Type header
        let response = Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(axum::body::Body::from(content.content))
            .unwrap();

        Ok(response)
    } else {
        // Content not found
        Err(ApiError::BadRequest(format!(
            "Content with ID {} not found",
            id
        )))
    }
}

pub enum ApiError {
    InternalError(ClassifyError),
    BadRequest(String),
    Conflict(ClassifyResponse),
}

impl From<ClassifyError> for ApiError {
    fn from(error: ClassifyError) -> Self {
        error!("API error: {}", error);
        Self::InternalError(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::InternalError(error) => {
                let body = Json(ContentQueryResponse {
                    items: Vec::new(),
                    tags: Vec::new(),
                    count: 0,
                    success: false,
                    error: Some(format!("Internal server error: {}", error)),
                });

                // Create response with explicit Content-Type header
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&body.0).unwrap(),
                    ))
                    .unwrap()
            }
            Self::BadRequest(message) => {
                let body = Json(ContentQueryResponse {
                    items: Vec::new(),
                    tags: Vec::new(),
                    count: 0,
                    success: false,
                    error: Some(message),
                });

                // Create response with explicit Content-Type header
                Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&body.0).unwrap(),
                    ))
                    .unwrap()
            }
            Self::Conflict(response) => {
                // Create response with explicit Content-Type header
                Response::builder()
                    .status(StatusCode::CONFLICT)
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&response).unwrap(),
                    ))
                    .unwrap()
            }
        }
    }
}
