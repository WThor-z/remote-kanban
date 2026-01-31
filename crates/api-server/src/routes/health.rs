//! Health check endpoint

use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    status: String,
    version: String,
    data_dir: String,
    worker_url: String,
    repo_path: String,
}

async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let data_dir = std::env::var("VK_DATA_DIR").unwrap_or_else(|_| ".vk-data".to_string());
    let worker_url = std::env::var("AGENT_WORKER_URL")
        .unwrap_or_else(|_| "http://localhost:4000".to_string());
    let repo_path = state.repo_path().to_string_lossy().to_string();

    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        data_dir,
        worker_url,
        repo_path,
    })
}

pub fn router() -> Router<AppState> {
    Router::new().route("/health", get(health_check))
}
