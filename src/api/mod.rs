
pub mod api {

  use axum::{
    extract::Query, routing::{get,post}, Router
  }; 
  use tower_http::trace::TraceLayer;
  use serde::Deserialize;
  use crate::llm::llm::query_llm;

  #[derive(Deserialize, Debug)]
  struct Text {
    t: String,
  }

  pub fn routes() -> Router {
    let health_router = Router::new().route("/", get(|| async {}));
    let api_router = api_routes();
    Router::new()
      .nest("/api", api_router)
      .nest("/health", health_router)
  }

  pub fn api_routes() -> Router {
    Router::new()
      .route("/", get(root))
      .route("/", post(process_text))
      .layer(TraceLayer::new_for_http())
  }
  
  async fn root(Query(params): Query<Vec<(String, String)>>) -> String { format!("Query-ing {:?}", params) }

  async fn process_text(Query(params): Query<Text>) -> String { 
    let text = params.t;
    let tags = query_llm(text);
    tags
   }
}

