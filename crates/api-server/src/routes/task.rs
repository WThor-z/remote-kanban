//! Task API endpoints
//!
//! RESTful API for task CRUD operations.

use agent_runner::{ChatMessage, ExecutionEvent, ExecutionStatus, RunSummary};
use axum::{
    extract::{Path, Query, State},
    http::{header::HeaderName, HeaderMap, HeaderValue, StatusCode},
    routing::{delete, get},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::set_header::SetResponseHeaderLayer;
use uuid::Uuid;

use vk_core::task::{Task, TaskPriority, TaskRepository, TaskStatus};

use crate::auth::{resolve_user_identity, UserIdentity};
use crate::feature_flags::feature_multi_tenant;
use crate::state::AppState;

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    pub title: String,
    #[serde(default)]
    pub project_id: Option<Uuid>,
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
    pub org_id: String,
    pub project_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
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
    pub project_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
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
pub struct ListTasksQuery {
    #[serde(default)]
    pub project_id: Option<Uuid>,
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
    #[serde(default)]
    pub org_id: Option<String>,
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
            project_id: run.project_id,
            workspace_id: run.workspace_id,
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
            org_id: task.org_id,
            project_id: task.project_id,
            workspace_id: task.workspace_id,
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
    headers: HeaderMap,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<Vec<TaskResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let identity = resolve_route_identity(&headers)?;
    let tasks = state.task_store().list().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let org_filter = identity
        .as_ref()
        .map(|current| current.org_id.as_str())
        .or(query.org_id.as_deref());

    Ok(Json(
        tasks
            .into_iter()
            .filter(|task| {
                let org_ok = org_filter.is_none_or(|org_id| task.org_id == org_id);
                let project_ok = query
                    .project_id
                    .is_none_or(|project_id| task.project_id == Some(project_id));
                let workspace_ok = query
                    .workspace_id
                    .is_none_or(|workspace_id| task.workspace_id == Some(workspace_id));
                project_ok && workspace_ok && org_ok
            })
            .map(TaskResponse::from)
            .collect(),
    ))
}

/// POST /api/tasks - Create a new task
async fn create_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<TaskResponse>), (StatusCode, Json<ErrorResponse>)> {
    let identity = resolve_route_identity(&headers)?;
    // Validate input
    if req.title.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Title cannot be empty".to_string(),
            }),
        ));
    }

    let project_id = req.project_id.ok_or_else(|| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: "Project is required".to_string(),
            }),
        )
    })?;

    let project = match state.project_store().get(project_id).await {
        Some(project) => project,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Project {} not found", project_id),
                }),
            ));
        }
    };
    ensure_project_visible(&project.org_id, identity.as_ref())?;

    let mut task = Task::new(req.title)
        .with_project_binding(project_id, project.workspace_id)
        .with_org_id(project.org_id);

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
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<ErrorResponse>)> {
    let identity = resolve_route_identity(&headers)?;
    let task = state.task_store().get(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    match task {
        Some(t) => {
            ensure_task_visible(&t, identity.as_ref())?;
            Ok(Json(TaskResponse::from(t)))
        }
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
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<RunSummaryResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let identity = resolve_route_identity(&headers)?;
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
    let task = task.unwrap();
    ensure_task_visible(&task, identity.as_ref())?;

    let runs = state.executor().list_runs(id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(
        runs.into_iter()
            .map(|run| {
                let mut summary = RunSummaryResponse::from(run);
                summary.project_id = summary.project_id.or(task.project_id);
                summary.workspace_id = summary.workspace_id.or(task.workspace_id);
                summary
            })
            .collect(),
    ))
}

/// DELETE /api/tasks/:id/runs - Delete all runs for a task
async fn delete_task_runs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let identity = resolve_route_identity(&headers)?;
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
    ensure_task_visible(&task.unwrap(), identity.as_ref())?;

    let runs = state.executor().list_runs(id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    if runs.iter().any(|run| run.status.is_active()) {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Run is active".to_string(),
            }),
        ));
    }

    state
        .executor()
        .run_store()
        .delete_task_runs(id)
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

/// GET /api/tasks/:id/runs/:run_id/events - List events for a run
async fn list_run_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((task_id, run_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<RunEventsQuery>,
) -> Result<Json<RunEventsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let identity = resolve_route_identity(&headers)?;
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
    ensure_task_visible(&task.unwrap(), identity.as_ref())?;

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

    let (events, has_more) = state
        .executor()
        .load_run_events(
            task_id,
            run_id,
            offset,
            limit,
            query.event_type,
            query.agent_event_type,
        )
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    let next_offset = if has_more {
        Some(offset + events.len())
    } else {
        None
    };

    Ok(Json(RunEventsResponse {
        events,
        has_more,
        next_offset,
    }))
}

/// GET /api/tasks/:id/runs/:run_id/messages - List messages for a run
async fn list_run_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((task_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<RunMessagesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let identity = resolve_route_identity(&headers)?;
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
    ensure_task_visible(&task.unwrap(), identity.as_ref())?;

    // Load messages from RunStore
    let messages = state
        .executor()
        .run_store()
        .load_messages(task_id, run_id)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(RunMessagesResponse { messages }))
}

/// DELETE /api/tasks/:id/runs/:run_id - Delete a run
async fn delete_run(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((task_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let identity = resolve_route_identity(&headers)?;
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
    ensure_task_visible(&task.unwrap(), identity.as_ref())?;

    let runs = state.executor().list_runs(task_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let run = runs.iter().find(|run| run.id == run_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Run {} not found", run_id),
            }),
        )
    })?;

    if run.status.is_active() {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Run is active".to_string(),
            }),
        ));
    }

    state
        .executor()
        .run_store()
        .delete_run(task_id, run_id)
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

/// PATCH /api/tasks/:id - Update a task
async fn update_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<ErrorResponse>)> {
    let identity = resolve_route_identity(&headers)?;
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
    ensure_task_visible(&task, identity.as_ref())?;

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
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let identity = resolve_route_identity(&headers)?;
    let task = state.task_store().get(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;
    let Some(task) = task else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Task {} not found", id),
            }),
        ));
    };
    ensure_task_visible(&task, identity.as_ref())?;

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

fn resolve_route_identity(
    headers: &HeaderMap,
) -> Result<Option<UserIdentity>, (StatusCode, Json<ErrorResponse>)> {
    resolve_user_identity(headers, feature_multi_tenant())
        .map_err(|error| (StatusCode::UNAUTHORIZED, Json(ErrorResponse { error })))
}

fn ensure_project_visible(
    project_org_id: &str,
    identity: Option<&UserIdentity>,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if let Some(identity) = identity {
        if project_org_id != identity.org_id {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Project not found".to_string(),
                }),
            ));
        }
    }
    Ok(())
}

fn ensure_task_visible(
    task: &Task,
    identity: Option<&UserIdentity>,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if let Some(identity) = identity {
        if task.org_id != identity.org_id {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Task {} not found", task.id),
                }),
            ));
        }
    }
    Ok(())
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
        .route(
            "/api/tasks/{id}/runs",
            get(list_task_runs).delete(delete_task_runs),
        )
        .route("/api/tasks/{id}/runs/{run_id}", delete(delete_run))
        .route("/api/tasks/{id}/runs/{run_id}/events", get(list_run_events))
        .route(
            "/api/tasks/{id}/runs/{run_id}/messages",
            get(list_run_messages),
        )
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("deprecation"),
            HeaderValue::from_static("true"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("link"),
            HeaderValue::from_static("</api/v1/executions>; rel=\"successor-version\""),
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use agent_runner::{AgentType, Run};
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use serde_json::{json, Value};
    use tempfile::TempDir;
    use tower::ServiceExt;
    use vk_core::kanban::KanbanStore;
    use vk_core::project::CreateProjectRequest;
    use vk_core::task::FileTaskStore;
    use vk_core::workspace::CreateWorkspaceRequest;

    use crate::gateway::GatewayManager;

    async fn build_state() -> (AppState, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let tasks_path = data_dir.join("tasks.json");
        let task_store = Arc::new(FileTaskStore::new(tasks_path).await.unwrap());
        let kanban_path = data_dir.join("kanban.json");
        let kanban_store = Arc::new(
            KanbanStore::with_task_store(kanban_path, Arc::clone(&task_store))
                .await
                .unwrap(),
        );
        let gateway_manager = Arc::new(GatewayManager::with_stores(
            Arc::clone(&task_store),
            Arc::clone(&kanban_store),
        ));
        let state = AppState::with_stores(
            data_dir,
            Arc::clone(&task_store),
            Arc::clone(&kanban_store),
            Arc::clone(&gateway_manager),
        )
        .await
        .unwrap();

        (state, temp_dir)
    }

    #[tokio::test]
    async fn list_tasks_supports_project_and_workspace_filters() {
        let (state, _temp_dir) = build_state().await;
        let workspace_1 = state.workspace_store().list().await[0].id;
        let workspace_2 = state
            .workspace_store()
            .create(CreateWorkspaceRequest {
                name: "Workspace Two".to_string(),
                slug: Some("workspace-two".to_string()),
                host_id: "host-two".to_string(),
                root_path: "/tmp/workspace-two".to_string(),
                default_project_id: None,
                org_id: None,
            })
            .await
            .unwrap()
            .id;

        let project_1 = state
            .project_store()
            .register(
                "host-one".to_string(),
                CreateProjectRequest {
                    name: "project-one".to_string(),
                    local_path: "/tmp/project-one".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id: workspace_1,
                    org_id: None,
                },
            )
            .await
            .unwrap();

        let project_2 = state
            .project_store()
            .register(
                "host-two".to_string(),
                CreateProjectRequest {
                    name: "project-two".to_string(),
                    local_path: "/tmp/project-two".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id: workspace_2,
                    org_id: None,
                },
            )
            .await
            .unwrap();

        let task_1 = state
            .task_store()
            .create(
                Task::new("Task one".to_string()).with_project_binding(project_1.id, workspace_1),
            )
            .await
            .unwrap();
        let task_2 = state
            .task_store()
            .create(
                Task::new("Task two".to_string()).with_project_binding(project_2.id, workspace_2),
            )
            .await
            .unwrap();
        state
            .task_store()
            .create(Task::new("Unbound task".to_string()))
            .await
            .unwrap();

        let app = router().with_state(state.clone());

        let by_project = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/tasks?projectId={}", project_1.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(by_project.status(), StatusCode::OK);
        let by_project_body = to_bytes(by_project.into_body(), usize::MAX).await.unwrap();
        let by_project_payload: Value = serde_json::from_slice(&by_project_body).unwrap();
        let by_project_items = by_project_payload.as_array().unwrap();
        assert_eq!(by_project_items.len(), 1);
        assert_eq!(by_project_items[0]["id"], task_1.id.to_string());

        let by_workspace = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/tasks?workspaceId={}", workspace_2))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(by_workspace.status(), StatusCode::OK);
        let by_workspace_body = to_bytes(by_workspace.into_body(), usize::MAX)
            .await
            .unwrap();
        let by_workspace_payload: Value = serde_json::from_slice(&by_workspace_body).unwrap();
        let by_workspace_items = by_workspace_payload.as_array().unwrap();
        assert_eq!(by_workspace_items.len(), 1);
        assert_eq!(by_workspace_items[0]["id"], task_2.id.to_string());

        let by_both = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/api/tasks?projectId={}&workspaceId={}",
                        project_1.id, workspace_1
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(by_both.status(), StatusCode::OK);
        let by_both_body = to_bytes(by_both.into_body(), usize::MAX).await.unwrap();
        let by_both_payload: Value = serde_json::from_slice(&by_both_body).unwrap();
        let by_both_items = by_both_payload.as_array().unwrap();
        assert_eq!(by_both_items.len(), 1);
        assert_eq!(by_both_items[0]["id"], task_1.id.to_string());
    }

    #[tokio::test]
    async fn list_runs_includes_project_and_workspace_context() {
        let (state, _temp_dir) = build_state().await;
        let task = state
            .task_store()
            .create(Task::new("List runs with context".to_string()))
            .await
            .unwrap();
        let project_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        let mut run = Run::new(
            task.id,
            AgentType::OpenCode,
            "Test prompt".to_string(),
            "main".to_string(),
        );
        run.metadata.project_id = Some(project_id);
        run.metadata.workspace_id = Some(workspace_id);
        run.update_status(ExecutionStatus::Completed);
        state.executor().run_store().save_run(&run).unwrap();

        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/tasks/{}/runs", task.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let listed_run = payload.as_array().unwrap().first().unwrap();

        assert_eq!(listed_run["projectId"], project_id.to_string());
        assert_eq!(listed_run["workspaceId"], workspace_id.to_string());
    }

    #[tokio::test]
    async fn list_runs_backfills_legacy_context_from_task_binding() {
        let (state, _temp_dir) = build_state().await;
        let project_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let task = state
            .task_store()
            .create(
                Task::new("Legacy run context".to_string())
                    .with_project_binding(project_id, workspace_id),
            )
            .await
            .unwrap();

        let mut run = Run::new(
            task.id,
            AgentType::OpenCode,
            "Legacy prompt".to_string(),
            "main".to_string(),
        );
        run.update_status(ExecutionStatus::Completed);
        state.executor().run_store().save_run(&run).unwrap();

        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/tasks/{}/runs", task.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let listed_run = payload.as_array().unwrap().first().unwrap();

        assert_eq!(listed_run["projectId"], project_id.to_string());
        assert_eq!(listed_run["workspaceId"], workspace_id.to_string());
    }

    #[tokio::test]
    async fn delete_run_returns_no_content() {
        let (state, _temp_dir) = build_state().await;
        let task = state
            .task_store()
            .create(Task::new("Delete run test".to_string()))
            .await
            .unwrap();

        let mut run = Run::new(
            task.id,
            AgentType::OpenCode,
            "Test prompt".to_string(),
            "main".to_string(),
        );
        run.update_status(ExecutionStatus::Completed);
        state.executor().run_store().save_run(&run).unwrap();

        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/tasks/{}/runs/{}", task.id, run.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(state.executor().list_runs(task.id).unwrap().is_empty());
    }

    #[tokio::test]
    async fn delete_run_missing_task_returns_not_found() {
        let (state, _temp_dir) = build_state().await;
        let task_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();

        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/tasks/{}/runs/{}", task_id, run_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_run_missing_run_returns_not_found() {
        let (state, _temp_dir) = build_state().await;
        let task = state
            .task_store()
            .create(Task::new("Delete run missing".to_string()))
            .await
            .unwrap();
        let run_id = Uuid::new_v4();

        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/tasks/{}/runs/{}", task.id, run_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_run_active_run_returns_conflict() {
        let (state, _temp_dir) = build_state().await;
        let task = state
            .task_store()
            .create(Task::new("Delete run active".to_string()))
            .await
            .unwrap();

        let mut run = Run::new(
            task.id,
            AgentType::OpenCode,
            "Test prompt".to_string(),
            "main".to_string(),
        );
        run.update_status(ExecutionStatus::Running);
        state.executor().run_store().save_run(&run).unwrap();

        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/tasks/{}/runs/{}", task.id, run.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn delete_task_runs_removes_all() {
        let (state, _temp_dir) = build_state().await;
        let task = state
            .task_store()
            .create(Task::new("Delete task runs".to_string()))
            .await
            .unwrap();

        let mut first_run = Run::new(
            task.id,
            AgentType::OpenCode,
            "Test prompt 1".to_string(),
            "main".to_string(),
        );
        first_run.update_status(ExecutionStatus::Completed);
        state.executor().run_store().save_run(&first_run).unwrap();

        let mut second_run = Run::new(
            task.id,
            AgentType::OpenCode,
            "Test prompt 2".to_string(),
            "main".to_string(),
        );
        second_run.update_status(ExecutionStatus::Completed);
        state.executor().run_store().save_run(&second_run).unwrap();

        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/tasks/{}/runs", task.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(state.executor().list_runs(task.id).unwrap().is_empty());
    }

    #[tokio::test]
    async fn delete_task_runs_rejects_active_runs() {
        let (state, _temp_dir) = build_state().await;
        let task = state
            .task_store()
            .create(Task::new("Delete task runs active".to_string()))
            .await
            .unwrap();

        let mut run = Run::new(
            task.id,
            AgentType::OpenCode,
            "Test prompt".to_string(),
            "main".to_string(),
        );
        run.update_status(ExecutionStatus::Running);
        state.executor().run_store().save_run(&run).unwrap();

        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/tasks/{}/runs", task.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn create_task_without_project_id_returns_unprocessable_entity() {
        let (state, _temp_dir) = build_state().await;

        let app = router().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("Content-Type", "application/json")
                    .body(Body::from(json!({ "title": "New task" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["error"], "Project is required");
    }

    #[tokio::test]
    async fn create_task_with_missing_project_returns_not_found() {
        let (state, _temp_dir) = build_state().await;

        let app = router().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "title": "New task",
                            "projectId": Uuid::new_v4(),
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn create_task_with_valid_project_sets_project_id() {
        let (state, _temp_dir) = build_state().await;
        let workspace_id = state.workspace_store().list().await[0].id;
        let project = state
            .project_store()
            .register(
                "host-test".to_string(),
                CreateProjectRequest {
                    name: "test-project".to_string(),
                    local_path: "/tmp/test-project".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id,
                    org_id: None,
                },
            )
            .await
            .unwrap();

        let app = router().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "title": "New task",
                            "projectId": project.id,
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["projectId"], project.id.to_string());
        assert_eq!(payload["workspaceId"], workspace_id.to_string());
    }

    #[tokio::test]
    async fn task_router_sets_deprecation_header() {
        let (state, _temp_dir) = build_state().await;
        let app = router().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/tasks")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let deprecation = response.headers().get("deprecation").unwrap();
        assert_eq!(deprecation, "true");
    }
}
