#[cfg(test)]
mod tests {
    use crate::api::AppState;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        response::Response,
        routing::{get, post},
        Router,
    };
    use mockall::mock;
    use mockall::predicate::*;
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::classifier::Classifier;
    use crate::storage::{ContentStorage, TagStorage};
    use crate::{ClassifyRequest, ClassifyResponse, ClassifyResult, Content, TagsResponse};

    // Mock Classifier
    mock! {
        pub ClassifierMock {}
        #[async_trait::async_trait]
        impl Classifier for ClassifierMock {
            async fn classify(&self, content: &str) -> ClassifyResult<Vec<String>>;
            async fn classify_url(&self, url: &str) -> ClassifyResult<Vec<String>>;
        }
    }

    // Mock ContentStorage
    mock! {
        pub ContentStorageMock {}
        #[async_trait::async_trait]
        impl ContentStorage for ContentStorageMock {
            async fn store(&self, content: &Content) -> ClassifyResult<()>;
            async fn get(&self, id: &str) -> ClassifyResult<Option<Content>>;
            async fn list(&self) -> ClassifyResult<Vec<Content>>;
            async fn delete(&self, id: &str) -> ClassifyResult<bool>;
            async fn find_by_hash(&self, hash: &str) -> ClassifyResult<Option<Content>>;
        }
    }

    // Mock TagStorage
    mock! {
        pub TagStorageMock {}
        #[async_trait::async_trait]
        impl TagStorage for TagStorageMock {
            async fn add_tags(&self, content_id: &str, tags: &[String]) -> ClassifyResult<()>;
            async fn get_tags(&self, content_id: &str) -> ClassifyResult<Vec<String>>;
            async fn list_tags(&self) -> ClassifyResult<Vec<String>>;
            async fn find_by_tag(&self, tag: &str) -> ClassifyResult<Vec<String>>;
            async fn remove_tags(&self, content_id: &str, tags: &[String]) -> ClassifyResult<()>;
        }
    }

    #[tokio::test]
    async fn test_classify_duplicate_content() {
        // Mock the config for testing
        let api_key = "test-api-key";

        // Set up test data
        let test_content = "Test content for duplicate detection";
        let content_hash = Content::generate_hash(test_content);
        let existing_content = Content::new(test_content.to_string())
            .with_tags(vec!["test".to_string(), "duplicate".to_string()]);

        let classifier_mock = MockClassifierMock::new();
        let mut content_storage_mock = MockContentStorageMock::new();
        let tag_storage_mock = MockTagStorageMock::new();

        content_storage_mock
            .expect_find_by_hash()
            .with(eq(content_hash))
            .times(1)
            .returning(move |_| Ok(Some(existing_content.clone())));

        let state = AppState {
            classifier: Arc::new(classifier_mock),
            content_storage: Arc::new(content_storage_mock),
            tag_storage: Arc::new(tag_storage_mock),
        };

        // Create router but without the API key validation middleware for testing
        let app = Router::new()
            .route("/classify", post(crate::api::classify_content))
            .with_state(Arc::new(state));

        let request = Request::post("/classify")
            .header("Content-Type", "application/json")
            .header("X-Api-Key", api_key)
            .body(Body::from(
                serde_json::to_string(&ClassifyRequest {
                    content: test_content.to_string(),
                })
                .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);

        let body = response_to_bytes(response).await;

        let response: ClassifyResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(response.content.content, test_content);
        assert!(response.content.tags.contains(&"test".to_string()));
        assert!(response.content.tags.contains(&"duplicate".to_string()));
        assert!(response.success);
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_get_tags() {
        // Mock the config for testing
        let api_key = "test-api-key";

        // Set up mocks
        let classifier_mock = MockClassifierMock::new();
        let content_storage_mock = MockContentStorageMock::new();
        let mut tag_storage_mock = MockTagStorageMock::new();

        // Mock the list_tags method
        let mock_tags = vec![
            "rust".to_string(),
            "programming".to_string(),
            "web".to_string(),
        ];

        tag_storage_mock
            .expect_list_tags()
            .times(1)
            .returning(move || Ok(mock_tags.clone()));

        // Create app state
        let state = AppState {
            classifier: Arc::new(classifier_mock),
            content_storage: Arc::new(content_storage_mock),
            tag_storage: Arc::new(tag_storage_mock),
        };

        // Create router but without the API key validation middleware for testing
        let app = Router::new()
            .route("/tags", get(crate::api::get_tags))
            .with_state(Arc::new(state));

        // Create request
        let request = Request::get("/tags")
            .header("X-Api-Key", api_key)
            .body(Body::empty())
            .unwrap();

        // Call the endpoint
        let response = app.oneshot(request).await.unwrap();

        // Verify response status
        assert_eq!(response.status(), StatusCode::OK);

        // Verify Content-Type header
        assert_eq!(
            response.headers().get("Content-Type").unwrap(),
            "application/json"
        );

        // Parse the response body
        let body = response_to_bytes(response).await;
        let response: TagsResponse = serde_json::from_slice(&body).unwrap();

        // Verify the response contains the expected tags
        assert_eq!(response.count, 3);
        assert!(response.tags.contains(&"rust".to_string()));
        assert!(response.tags.contains(&"programming".to_string()));
        assert!(response.tags.contains(&"web".to_string()));
        assert!(response.success);
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_get_content_text() {
        // Mock the config for testing
        let api_key = "test-api-key";
        let content_id = "test-content-id";
        let test_content = "This is the content text for testing";

        // Set up mocks
        let classifier_mock = MockClassifierMock::new();
        let mut content_storage_mock = MockContentStorageMock::new();
        let tag_storage_mock = MockTagStorageMock::new();

        // Create a test content item
        let content = Content::new(test_content.to_string()).with_tags(vec!["test".to_string()]);

        // Mock the get method
        content_storage_mock
            .expect_get()
            .with(eq(content_id))
            .times(1)
            .returning(move |_| Ok(Some(content.clone())));

        // Create app state
        let state = AppState {
            classifier: Arc::new(classifier_mock),
            content_storage: Arc::new(content_storage_mock),
            tag_storage: Arc::new(tag_storage_mock),
        };

        // Create router without middleware for testing
        let app = Router::new()
            .route("/content/:id", get(crate::api::get_content_text))
            .with_state(Arc::new(state));

        // Create request
        let request = Request::get(&format!("/content/{}", content_id))
            .header("X-Api-Key", api_key)
            .body(Body::empty())
            .unwrap();

        // Call the endpoint
        let response = app.oneshot(request).await.unwrap();

        // Verify response status
        assert_eq!(response.status(), StatusCode::OK);

        // Verify Content-Type header
        assert_eq!(
            response.headers().get("Content-Type").unwrap(),
            "text/plain; charset=utf-8"
        );

        // Parse the response body as plain text
        let body = response_to_bytes(response).await;
        let text = String::from_utf8(body).unwrap();

        // Verify the response contains the content text
        assert_eq!(text, test_content);
    }

    async fn response_to_bytes(response: Response) -> Vec<u8> {
        let response_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec();
        response_bytes
    }
}
