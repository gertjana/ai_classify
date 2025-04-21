#[cfg(test)]
mod tests {
    use crate::api::{create_router, AppState};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        response::Response,
    };
    use mockall::mock;
    use mockall::predicate::*;
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::classifier::Classifier;
    use crate::storage::{ContentStorage, TagStorage};
    use crate::{ClassifyRequest, ClassifyResponse, ClassifyResult, Content};

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

        let app = create_router(state);

        let request = Request::post("/classify")
            .header("Content-Type", "application/json")
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

    async fn response_to_bytes(response: Response) -> Vec<u8> {
        let response_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec();
        response_bytes
    }
}
