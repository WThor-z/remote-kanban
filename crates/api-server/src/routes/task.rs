//! Task API endpoints
//!
//! RESTful API for task CRUD operations.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use agent_runner::{ChatMessage, ExecutionEvent, ExecutionStatus, RunSummary};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use vk_core::task::{Task, TaskPriority, TaskRepository, TaskStatus};

use crate::state::AppState;

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub priority: Option<TaskPriority>,
    #[serde(default)]
    pub agent_type: Option<String>,
    #[serde(default)]
    pub base_branch: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<TaskStatus>,
    #[serde(default)]
    pub priority: Option<TaskPriority>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResponse {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub agent_type: Option<String>,
    pub base_branch: Option<String>,
    pub model: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummaryResponse {
    pub id: Uuid,
    pub task_id: Uuid,
    pub agent_type: String,
    pub prompt_preview: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub status: ExecutionStatus,
    pub event_count: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunEventsQuery {
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub agent_event_type: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunEventsResponse {
    pub events: Vec<ExecutionEvent>,
    pub has_more: bool,
    pub next_offset: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunMessagesResponse {
    pub messages: Vec<ChatMessage>,
}

impl From<RunSummary> for RunSummaryResponse {
    fn from(run: RunSummary) -> Self {
        Self {
            id: run.id,
            task_id: run.task_id,
            agent_type: run.agent_type.as_str().to_string(),
            prompt_preview: run.prompt_preview,
            created_at: run.created_at.to_rfc3339(),
            started_at: run.started_at.map(|t| t.to_rfc3339()),
            ended_at: run.ended_at.map(|t| t.to_rfc3339()),
            duration_ms: run.duration_ms,
            status: run.status,
            event_count: run.event_count,
        }
    }
}

impl From<Task> for TaskResponse {
    fn from(task: Task) -> Self {
        Self {
            id: task.id,
            title: task.title,
            description: task.description,
            status: task.status,
            priority: task.priority,
            agent_type: task.agent_type,
            base_branch: task.base_branch,
            model: task.model,
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/tasks - List all tasks
async fn list_tasks(
    State(state): State<AppState>,
) -> Result<Json<Vec<TaskResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let tasks = state.task_store().list().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(tasks.into_iter().map(TaskResponse::from).collect()))
}

/// POST /api/tasks - Create a new task
async fn create_task(
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<TaskResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Validate input
    if req.title.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Title cannot be empty".to_string(),
            }),
        ));
    }

    let mut task = Task::new(req.title);

    if let Some(desc) = req.description {
        task = task.with_description(desc);
    }

    if let Some(priority) = req.priority {
        task = task.with_priority(priority);
    }

    if let Some(agent_type) = req.agent_type {
        task = task.with_agent_type(agent_type);
    }

    if let Some(base_branch) = req.base_branch {
        task = task.with_base_branch(base_branch);
    }

    if let Some(model) = req.model {
        task = task.with_model(model);
    }

    let created = state.task_store().create(task).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok((StatusCode::CREATED, Json(TaskResponse::from(created))))
}

/// GET /api/tasks/:id - Get a single task
async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<ErrorResponse>)> {
    let task = state.task_store().get(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    match task {
        Some(t) => Ok(Json(TaskResponse::from(t))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Task {} not found", id),
            }),
        )),
    }
}

/// GET /api/tasks/:id/runs - List all runs for a task
async fn list_task_runs(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<RunSummaryResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let task = state.task_store().get(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    if task.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Task {} not found", id),
            }),
        ));
    }

    let runs = state.executor().list_runs(id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(
        runs.into_iter().map(RunSummaryResponse::from).collect(),
    ))
}

/// GET /api/tasks/:id/runs/:run_id/events - List events for a run
async fn list_run_events(
    State(state): State<AppState>,
    Path((task_id, run_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<RunEventsQuery>,
) -> Result<Json<RunEventsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let task = state.task_store().get(task_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    if task.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Task {} not found", task_id),
            }),
        ));
    }

    let runs = state.executor().list_runs(task_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    if !runs.iter().any(|run| run.id == run_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Run {} not found", run_id),
            }),
        ));
    }

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(200).min(1000);

    let (events, has_more) = state.executor().load_run_events(
        task_id,
        run_id,
        offset,
        limit,
        query.event_type,
        query.agent_event_type,
    ).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let next_offset = if has_more { Some(offset + events.len()) } else { None };

    Ok(Json(RunEventsResponse {
        events,
        has_more,
        next_offset,
    }))
}

/// GET /api/tasks/:id/runs/:run_id/messages - List messages for a run
async fn list_run_messages(
    State(state): State<AppState>,
    Path((task_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<RunMessagesResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify task exists
    let task = state.task_store().get(task_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    if task.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Task {} not found", task_id),
            }),
        ));
    }

    // Load messages from RunStore
    let messages = state.executor().run_store().load_messages(task_id, run_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(RunMessagesResponse { messages }))
}

/// PATCH /api/tasks/:id - Update a task
async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<ErrorResponse>)> {
    // First get the existing task
    let existing = state.task_store().get(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let mut task = match existing {
        Some(t) => t,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Task {} not found", id),
                }),
            ))
        }
    };

    // Apply updates
    if let Some(title) = req.title {
        if title.trim().is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Title cannot be empty".to_string(),
                }),
            ));
        }
        task.title = title;
    }

    if let Some(desc) = req.description {
        task.description = Some(desc);
    }

    if let Some(status) = req.status {
        task.status = status;
    }

    if let Some(priority) = req.priority {
        task.priority = priority;
    }

    let updated = state.task_store().update(task).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(TaskResponse::from(updated)))
}

/// DELETE /api/tasks/:id - Delete a task
async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state.task_store().delete(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Task {} not found", id),
            }),
        ))
    }
}

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/tasks", get(list_tasks).post(create_task))
        .route(
            "/api/tasks/{id}",
            get(get_task).patch(update_task).delete(delete_task),
        )
        .route("/api/tasks/{id}/runs", get(list_task_runs))
        .route("/api/tasks/{id}/runs/{run_id}/events", get(list_run_events))
        .route("/api/tasks/{id}/runs/{run_id}/messages", get(list_run_messages))
}
