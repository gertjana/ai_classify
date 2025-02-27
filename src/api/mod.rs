pub mod api {

    use crate::llm::llm::query_llm;
    use axum::{
        extract::{Query, State},
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

    pub fn routes(config: Config) -> Router {
        let health_router = Router::new().route("/", get(|| async {}));
        Router::new()
            .nest("/api", api_routes())
            .nest("/health", health_router)
            .with_state(config)
    }

    pub fn api_routes() -> Router<Config> {
        Router::new()
            .route("/", get(root))
            .route("/", post(process_text))
            .layer(TraceLayer::new_for_http())
    }

    async fn root(State(_): State<Config>, Query(params): Query<Vec<(String, String)>>) -> String {
        format!("Query-ing {:?}", params)
    }

    async fn process_text(State(config): State<Config>, Query(params): Query<Text>) -> String {
        let text = params.t;
        let tags = query_llm(text, config);
        tags
    }
}
