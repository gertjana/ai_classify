use axum;
use crate::api::api::routes;

mod api;
mod llm;

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();


  let app = routes();

  let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
  axum::serve(listener, app).await.unwrap();
}