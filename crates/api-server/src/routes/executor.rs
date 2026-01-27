//! Executor API endpoints
//!
//! RESTful API for task execution operations.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use agent_runner::{ExecuteRequest, SessionState};
use vk_core::task::TaskRepository;

use crate::state::AppState;

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartExecutionRequest {
    /// Agent type (opencode, claude-code, gemini-cli, codex)
    pub agent_type: String,
    /// Base branch to create worktree from
    pub base_branch: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResponse {
    pub session_id: Uuid,
    pub task_id: Uuid,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    pub session_id: Uuid,
    pub task_id: Uuid,
    pub status: String,
    pub state: String,
    pub worktree_path: Option<String>,
    pub branch: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionListResponse {
    pub sessions: Vec<SessionSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub session_id: Uuid,
    pub task_id: Uuid,
    pub state: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /api/tasks/:id/execute - Start task execution
async fn start_execution(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    Json(req): Json<StartExecutionRequest>,
) -> Result<(StatusCode, Json<ExecutionResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Verify task exists
    let task = state
        .task_store()
        .get(task_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Task {} not found", task_id),
                }),
            )
        })?;

    // Build prompt from task
    let prompt = if let Some(desc) = &task.description {
        format!("{}\n\n{}", task.title, desc)
    } else {
        task.title.clone()
    };

    // Create execute request
    let execute_req = ExecuteRequest {
        task_id,
        agent_type: req.agent_type,
        base_branch: req.base_branch,
        prompt,
    };

    // Start execution
    let (session_id, _event_rx) = state.executor().execute(execute_req).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    // TODO: Store event_rx for WebSocket streaming

    Ok((
        StatusCode::ACCEPTED,
        Json(ExecutionResponse {
            session_id,
            task_id,
            status: "started".to_string(),
            message: "Execution started successfully".to_string(),
        }),
    ))
}

/// GET /api/tasks/:id/status - Get execution status
async fn get_execution_status(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<SessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let session = state
        .executor()
        .get_session_by_task(task_id)
        .await
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("No active session for task {}", task_id),
                }),
            )
        })?;

    let session = session.read().await;
    let session_state = session.state().await;
    let status = session.status().await;

    Ok(Json(SessionResponse {
        session_id: session.id,
        task_id: session.task_id,
        status: format!("{:?}", status).to_lowercase(),
        state: state_to_string(&session_state),
        worktree_path: session.worktree_path().map(|p| p.to_string_lossy().to_string()),
        branch: session.worktree.as_ref().map(|w| w.branch.clone()),
    }))
}

/// POST /api/tasks/:id/stop - Stop task execution
async fn stop_execution(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<ExecutionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let session = state
        .executor()
        .get_session_by_task(task_id)
        .await
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("No active session for task {}", task_id),
                }),
            )
        })?;

    let session_id = session.read().await.id;

    state.executor().cancel_session(session_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(ExecutionResponse {
        session_id,
        task_id,
        status: "cancelled".to_string(),
        message: "Execution cancelled".to_string(),
    }))
}

/// DELETE /api/tasks/:id/worktree - Clean up worktree
async fn cleanup_worktree(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let session = state
        .executor()
        .get_session_by_task(task_id)
        .await
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("No session for task {}", task_id),
                }),
            )
        })?;

    let session_id = session.read().await.id;

    state
        .executor()
        .cleanup_session(session_id, true)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/sessions - List all sessions
async fn list_sessions(
    State(state): State<AppState>,
) -> Result<Json<SessionListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let sessions = state.executor().list_sessions().await;

    let summaries: Vec<SessionSummary> = sessions
        .into_iter()
        .map(|(session_id, task_id, session_state)| SessionSummary {
            session_id,
            task_id,
            state: state_to_string(&session_state),
        })
        .collect();

    Ok(Json(SessionListResponse { sessions: summaries }))
}

/// GET /api/sessions/:id - Get session details
async fn get_session(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<SessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let session = state
        .executor()
        .get_session(session_id)
        .await
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session {} not found", session_id),
                }),
            )
        })?;

    let session = session.read().await;
    let session_state = session.state().await;
    let status = session.status().await;

    Ok(Json(SessionResponse {
        session_id: session.id,
        task_id: session.task_id,
        status: format!("{:?}", status).to_lowercase(),
        state: state_to_string(&session_state),
        worktree_path: session.worktree_path().map(|p| p.to_string_lossy().to_string()),
        branch: session.worktree.as_ref().map(|w| w.branch.clone()),
    }))
}

// ============================================================================
// Helpers
// ============================================================================

fn state_to_string(state: &SessionState) -> String {
    match state {
        SessionState::Pending => "pending".to_string(),
        SessionState::Initializing => "initializing".to_string(),
        SessionState::Running { .. } => "running".to_string(),
        SessionState::Paused => "paused".to_string(),
        SessionState::Completed { exit_code, .. } => format!("completed({})", exit_code),
        SessionState::Failed { error, .. } => format!("failed: {}", error),
        SessionState::Cancelled { .. } => "cancelled".to_string(),
    }
}

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        // Task execution endpoints
        .route("/api/tasks/{id}/execute", post(start_execution))
        .route("/api/tasks/{id}/status", get(get_execution_status))
        .route("/api/tasks/{id}/stop", post(stop_execution))
        .route("/api/tasks/{id}/worktree", delete(cleanup_worktree))
        // Session endpoints
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions/{id}", get(get_session))
}
