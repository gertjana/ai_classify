pub mod api {

    use crate::llm::llm::query_llm;
    use crate::storage::storage::{get_tags, get_texts, store_tags};
    use axum::body::Body;
    use axum::Json;
    use axum::{
        extract::{Query, State},
        http::{Response, StatusCode},
        response::IntoResponse,
        routing::{get, post},
        Router,
    };
    use config::Config;
    use serde::Deserialize;
    use tower_http::trace::TraceLayer;

    #[derive(Deserialize, Debug)]
    struct Text {
        t: String,
    }

    #[derive(Deserialize, Debug)]
    struct Tag {
        q: String,
    }

    pub fn routes(config: Config) -> Router {
        let health_router = Router::new().route("/", get(|| async {}));
        Router::new()
            .nest("/api", api_routes())
            .nest("/health", health_router)
            .with_state(config)
    }

    pub fn api_routes() -> Router<Config> {
        Router::new()
            .route("/", get(query_tag))
            .route("/", post(process_text))
            .route("/tags", get(get_all_tags))
            .layer(TraceLayer::new_for_http())
    }

    async fn query_tag(
        State(config): State<Config>,
        Query(params): Query<Tag>,
    ) -> Result<Json<Vec<String>>, axum::response::Response> {
        let tag = params.q;
        let user_id = &config.get_string("general.user-id").unwrap();
        let texts =
            get_texts(user_id.clone(), tag, &config).map_err(INTERNAL_SERVER_ERR_RESPONSE)?;
        Ok(Json(texts))
    }

    async fn process_text(
        State(config): State<Config>,
        Query(params): Query<Text>,
    ) -> Result<Json<Vec<String>>, axum::response::Response> {
        let text = params.t;
        let user_id = &config.get_string("general.user-id").unwrap();
        let tags = query_llm(text.clone(), &config).map_err(INTERNAL_SERVER_ERR_RESPONSE)?;
        let tags_as_vec: Vec<String> = tags.split(",").map(|s| s.to_string()).collect();
        store_tags(
            user_id.clone(),
            text,
            tags_as_vec.clone(),
            &config,
        )
        .map_err(INTERNAL_SERVER_ERR_RESPONSE)?;
        Ok(Json(tags_as_vec))
    }

    async fn get_all_tags(
        State(config): State<Config>,
    ) -> Result<Json<Vec<String>>, axum::response::Response> {
        let user_id = &config.get_string("general.user-id").unwrap();
        let tags = get_tags(user_id.clone(), &config).map_err(INTERNAL_SERVER_ERR_RESPONSE)?;
        Ok(Json(tags))
    }

    static INTERNAL_SERVER_ERR_RESPONSE: fn(anyhow::Error) -> Response<Body> =
        |e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
}
