use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
use serde::Serialize;

use crate::{api, get_port};

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

/// Create the API router
fn create_router() -> Router {
    Router::new().route("/health", get(health_handler))
}

/// Health check handler
async fn health_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }),
    )
}

/// Run the API server
pub fn run_healthcheck_api() {
    // Get port for health check server
    let port = get_port();

    let router = api::create_router();

    tracing::info!("Health check API listening on port {}", port);

    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
            .await
            .expect("Failed to run api");

        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!("API server error: {}", e);
        }
    });
}
