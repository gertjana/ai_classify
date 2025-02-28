use crate::api::api::routes;
use axum;
use config::Config;

mod api;
mod llm;
mod storage;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(true)
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing::Level::INFO.into())
                .parse_lossy("openai_api_rust::requests=warn"),
        )
        .init();

    let config = Config::builder()
        .add_source(config::File::with_name("config.toml"))
        .add_source(config::Environment::with_prefix("APP"))
        .build()
        .unwrap();

    let app = routes(config.clone());

    let port = config.get::<u16>("server.port").unwrap_or(3000);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
