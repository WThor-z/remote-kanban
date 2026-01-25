//! Task API endpoints
//!
//! Placeholder for task CRUD operations.
//! Will be fully implemented in M-1.2.

use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
struct TasksResponse {
    message: String,
}

async fn list_tasks() -> Json<TasksResponse> {
    Json(TasksResponse {
        message: "Task API coming soon (M-1.2)".to_string(),
    })
}

pub fn router() -> Router<AppState> {
    Router::new().route("/api/tasks", get(list_tasks))
}
