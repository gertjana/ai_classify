use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, error};

use crate::{ClassifyError, ClassifyRequest, ClassifyResponse, Content, ContentQueryResponse};
use crate::classifier::Classifier;
use crate::storage::{ContentStorage, TagStorage};

/// API server state
pub struct AppState {
    /// Content classifier
    pub classifier: Arc<dyn Classifier>,
    /// Content storage
    pub content_storage: Arc<dyn ContentStorage>,
    /// Tag storage
    pub tag_storage: Arc<dyn TagStorage>,
}

impl AppState {
    /// Create a new app state
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

/// Query parameters for content search
#[derive(Debug, Deserialize)]
pub struct QueryParams {
    /// Tags to search for (comma-separated)
    pub tags: String,
}

/// Response for delete operation
#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    /// Whether the deletion was successful
    pub success: bool,
    /// ID of the deleted content
    pub id: Option<String>,
    /// Tags that were removed (orphaned tags)
    pub removed_tags: Vec<String>,
    /// Any error message
    pub error: Option<String>,
}

/// Create the API router
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(health_check))
        .route("/classify", post(classify_content))
        .route("/query", get(query_content))
        .route("/content/:id", delete(delete_content))
        .with_state(Arc::new(state))
}

/// Start the API server
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
async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

/// Classify content endpoint
async fn classify_content(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ClassifyRequest>,
) -> Result<Json<ClassifyResponse>, ApiError> {
    info!("Received classification request");

    // Create content object
    let content = Content::new(request.content.clone());

    // Detect if content is a URL and classify accordingly
    let tags = if content.is_url() {
        info!("Detected URL: {}", &content.content);
        state.classifier.classify_url(&content.content).await?
    } else {
        info!("Detected text content");
        state.classifier.classify(&content.content).await?
    };

    // Add tags to content
    let content = content.with_tags(tags.clone());

    // Store content
    state.content_storage.store(&content).await?;

    // Store tags
    state.tag_storage.add_tags(&content.id.to_string(), &tags).await?;

    // Return response
    let response = ClassifyResponse {
        content,
        success: true,
        error: None,
    };

    Ok(Json(response))
}

/// Query content by tags endpoint
async fn query_content(
    State(state): State<Arc<AppState>>,
    Query(params): Query<QueryParams>,
) -> Result<Json<ContentQueryResponse>, ApiError> {
    info!("Received content query request for tags: {}", params.tags);

    // Parse tags from query string
    let tags: Vec<String> = params.tags
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if tags.is_empty() {
        return Err(ApiError::BadRequest("No valid tags provided".to_string()));
    }

    // Find all content IDs that match any of the provided tags
    let mut content_ids = HashSet::new();

    for tag in &tags {
        let tag_content_ids = state.tag_storage.find_by_tag(tag).await?;

        // Add all content IDs for this tag (union operation)
        for id in tag_content_ids {
            content_ids.insert(id);
        }
    }

    info!("Found {} content items matching the tags", content_ids.len());

    // Retrieve content for found IDs
    let mut items = Vec::new();
    for content_id in content_ids {
        if let Some(content) = state.content_storage.get(&content_id).await? {
            items.push(content);
        }
    }

    info!("Retrieved {} content items", items.len());

    // Sort by most recently updated first
    items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    // Calculate count before moving items
    let count = items.len();

    // Return response
    let response = ContentQueryResponse {
        items,
        tags,
        count,
        success: true,
        error: None,
    };

    Ok(Json(response))
}

/// Delete content endpoint
async fn delete_content(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DeleteResponse>, ApiError> {
    info!("Received delete content request for ID: {}", id);

    // Check if content exists
    if let Some(_) = state.content_storage.get(&id).await? {
        // Get the tags for this content before deletion
        let tags = state.tag_storage.get_tags(&id).await?;
        info!("Content has {} tags that may need cleanup", tags.len());

        // Delete the content
        let deleted = state.content_storage.delete(&id).await?;

        if !deleted {
            return Err(ApiError::BadRequest(format!("Failed to delete content with ID: {}", id)));
        }

        // Track which tags are orphaned and should be removed
        let mut orphaned_tags = Vec::new();

        // For each tag, check if it's used by any other content
        for tag in &tags {
            let content_with_tag = state.tag_storage.find_by_tag(tag).await?;

            // After removal of this content, if no other content has this tag, we can remove it entirely
            if content_with_tag.is_empty() {
                info!("Tag '{}' is now orphaned, will be removed", tag);
                orphaned_tags.push(tag.clone());
            }
        }

        // Remove the content's association with all its tags
        state.tag_storage.remove_tags(&id, &tags).await?;

        // Return response with information about orphaned tags
        let response = DeleteResponse {
            success: true,
            id: Some(id),
            removed_tags: orphaned_tags,
            error: None,
        };

        Ok(Json(response))
    } else {
        Err(ApiError::BadRequest(format!("Content with ID {} not found", id)))
    }
}

/// API error type
pub enum ApiError {
    /// Internal server error
    InternalError(ClassifyError),
    /// Bad request
    BadRequest(String),
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

                (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
            }
            Self::BadRequest(message) => {
                let body = Json(ContentQueryResponse {
                    items: Vec::new(),
                    tags: Vec::new(),
                    count: 0,
                    success: false,
                    error: Some(message),
                });

                (StatusCode::BAD_REQUEST, body).into_response()
            }
        }
    }
}
