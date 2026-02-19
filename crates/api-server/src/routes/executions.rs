//! Execution Orchestrator v1 API endpoints
//!
//! Adds execution-centric orchestration APIs while keeping `/api/tasks/*` compatibility.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::warn;
use uuid::Uuid;

use agent_runner::{ExecutionEvent, Run};

use crate::audit::AuditEvent;
use crate::auth::{resolve_user_identity, UserIdentity};
use crate::feature_flags::feature_multi_tenant;
use crate::state::AppState;

use super::executor::{
    start_task_execution, ErrorResponse, RouteError, StartTaskExecutionCommand,
    StartTaskExecutionResult,
};

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateExecutionRequest {
    pub task_id: Uuid,
    #[serde(default)]
    pub agent_type: Option<String>,
    #[serde(default)]
    pub base_branch: Option<String>,
    #[serde(default)]
    pub target_host: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub org_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionInputRequest {
    pub content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionEventsQuery {
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub agent_event_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListExecutionsQuery {
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub org_id: Option<String>,
    #[serde(default)]
    pub host_id: Option<String>,
    #[serde(default)]
    pub task_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateExecutionResponse {
    pub execution_id: Uuid,
    pub task_id: Uuid,
    pub status: String,
    pub message: String,
    pub host_id: String,
    pub trace_id: String,
    pub org_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionDetailResponse {
    pub execution_id: Uuid,
    pub task_id: Uuid,
    pub project_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub host_id: Option<String>,
    pub trace_id: Option<String>,
    pub org_id: Option<String>,
    pub parent_execution_id: Option<Uuid>,
    pub agent_role: Option<String>,
    pub handoff_id: Option<String>,
    pub agent_type: String,
    pub base_branch: String,
    pub status: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub summary: Option<String>,
    pub error: Option<String>,
    pub event_count: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionActionResponse {
    pub execution_id: Uuid,
    pub task_id: Uuid,
    pub status: String,
    pub accepted: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionEventEnvelope {
    pub execution_id: Uuid,
    pub org_id: String,
    pub trace_id: String,
    pub seq: u64,
    pub ts: i64,
    pub task_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_id: Option<String>,
    pub payload: ExecutionEvent,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionEventsResponse {
    pub events: Vec<ExecutionEventEnvelope>,
    pub has_more: bool,
    pub next_offset: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListExecutionsResponse {
    pub items: Vec<ExecutionDetailResponse>,
    pub has_more: bool,
    pub next_offset: Option<usize>,
}

impl From<StartTaskExecutionResult> for CreateExecutionResponse {
    fn from(value: StartTaskExecutionResult) -> Self {
        Self {
            execution_id: value.execution_id,
            task_id: value.task_id,
            status: value.status,
            message: value.message,
            host_id: value.host_id,
            trace_id: value.trace_id,
            org_id: value.org_id,
        }
    }
}

impl From<&Run> for ExecutionDetailResponse {
    fn from(run: &Run) -> Self {
        Self {
            execution_id: run.id,
            task_id: run.task_id,
            project_id: run.metadata.project_id,
            workspace_id: run.metadata.workspace_id,
            host_id: run.metadata.host_id.clone(),
            trace_id: run.metadata.trace_id.clone(),
            org_id: run.metadata.org_id.clone(),
            parent_execution_id: run.metadata.parent_execution_id,
            agent_role: run.metadata.agent_role.clone(),
            handoff_id: run.metadata.handoff_id.clone(),
            agent_type: run.agent_type.as_str().to_string(),
            base_branch: run.base_branch.clone(),
            status: format!("{:?}", run.status).to_lowercase(),
            created_at: run.created_at.to_rfc3339(),
            started_at: run.started_at.map(|ts| ts.to_rfc3339()),
            ended_at: run.ended_at.map(|ts| ts.to_rfc3339()),
            duration_ms: run.duration_ms,
            summary: run.summary.clone(),
            error: run.error.clone(),
            event_count: run.event_count,
        }
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/v1/executions - List executions for operations console
async fn list_executions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListExecutionsQuery>,
) -> Result<Json<ListExecutionsResponse>, RouteError> {
    let identity = resolve_user_identity(&headers, feature_multi_tenant()).map_err(unauthorized)?;
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let org_filter = identity
        .as_ref()
        .map(|current| current.org_id.clone())
        .or(query.org_id.clone());

    let (runs, has_more) = state
        .executor()
        .run_store()
        .list_runs_global_filtered_paginated(
            offset,
            limit,
            query.status.as_deref(),
            org_filter.as_deref(),
            query.host_id.as_deref(),
            query.task_id,
        )
        .map_err(internal_error)?;

    let items = runs
        .iter()
        .map(ExecutionDetailResponse::from)
        .collect::<Vec<_>>();
    let next_offset = if has_more {
        Some(offset + items.len())
    } else {
        None
    };

    Ok(Json(ListExecutionsResponse {
        items,
        has_more,
        next_offset,
    }))
}

/// POST /api/v1/executions - Create execution
async fn create_execution(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateExecutionRequest>,
) -> Result<(StatusCode, Json<CreateExecutionResponse>), RouteError> {
    let identity = resolve_user_identity(&headers, feature_multi_tenant()).map_err(unauthorized)?;
    if let (Some(current), Some(req_org_id)) = (identity.as_ref(), req.org_id.as_deref()) {
        if current.org_id != req_org_id {
            return Err(conflict_error(
                "orgId does not match authenticated organization",
            ));
        }
    }
    let org_id = identity
        .as_ref()
        .map(|current| current.org_id.clone())
        .or(req.org_id.clone());

    let result = start_task_execution(
        &state,
        StartTaskExecutionCommand {
            task_id: req.task_id,
            agent_type: req.agent_type.unwrap_or_else(|| "opencode".to_string()),
            base_branch: req.base_branch,
            target_host: req.target_host,
            model: req.model,
            trace_id: req.trace_id,
            org_id,
        },
    )
    .await?;

    record_audit_event(
        &state,
        AuditEvent::new(
            result.org_id.clone(),
            "api",
            "execution.created",
            Some(result.execution_id),
            Some(result.task_id),
            Some(result.host_id.clone()),
            Some(result.trace_id.clone()),
            Some(result.status.clone()),
            json!({ "source": "/api/v1/executions" }),
        ),
    )
    .await;

    Ok((
        StatusCode::ACCEPTED,
        Json(CreateExecutionResponse::from(result)),
    ))
}

/// GET /api/v1/executions/:id - Fetch execution details
async fn get_execution(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(execution_id): Path<Uuid>,
) -> Result<Json<ExecutionDetailResponse>, RouteError> {
    let identity = resolve_user_identity(&headers, feature_multi_tenant()).map_err(unauthorized)?;
    let run = state
        .executor()
        .run_store()
        .find_run(execution_id)
        .map_err(internal_error)?
        .ok_or_else(|| not_found(format!("Execution {} not found", execution_id)))?;
    ensure_run_visible_to_identity(&run, identity.as_ref())?;

    Ok(Json(ExecutionDetailResponse::from(&run)))
}

/// POST /api/v1/executions/:id/input - Send runtime input
async fn input_execution(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(execution_id): Path<Uuid>,
    Json(req): Json<ExecutionInputRequest>,
) -> Result<(StatusCode, Json<ExecutionActionResponse>), RouteError> {
    let identity = resolve_user_identity(&headers, feature_multi_tenant()).map_err(unauthorized)?;
    let run = state
        .executor()
        .run_store()
        .find_run(execution_id)
        .map_err(internal_error)?
        .ok_or_else(|| not_found(format!("Execution {} not found", execution_id)))?;
    ensure_run_visible_to_identity(&run, identity.as_ref())?;

    if run.is_terminal() {
        let terminal_status = format!("{:?}", run.status).to_lowercase();
        record_audit_event(
            &state,
            AuditEvent::new(
                run.metadata.org_id.clone().unwrap_or_else(default_org_id),
                "api",
                "execution.input.ignored",
                Some(run.id),
                Some(run.task_id),
                run.metadata.host_id.clone(),
                run.metadata.trace_id.clone(),
                Some(terminal_status.clone()),
                json!({ "reason": "terminal_state", "source": "/api/v1/executions/{id}/input" }),
            ),
        )
        .await;
        return Ok((
            StatusCode::ACCEPTED,
            Json(ExecutionActionResponse {
                execution_id: run.id,
                task_id: run.task_id,
                status: terminal_status,
                accepted: false,
                message: "Execution already ended; input ignored".to_string(),
            }),
        ));
    }

    match state
        .gateway_manager()
        .send_input(&run.task_id.to_string(), req.content)
        .await
    {
        Ok(()) => {
            record_audit_event(
                &state,
                AuditEvent::new(
                    run.metadata.org_id.clone().unwrap_or_else(default_org_id),
                    "api",
                    "execution.input.forwarded",
                    Some(run.id),
                    Some(run.task_id),
                    run.metadata.host_id.clone(),
                    run.metadata.trace_id.clone(),
                    Some("running".to_string()),
                    json!({ "source": "/api/v1/executions/{id}/input" }),
                ),
            )
            .await;
            Ok((
                StatusCode::ACCEPTED,
                Json(ExecutionActionResponse {
                    execution_id: run.id,
                    task_id: run.task_id,
                    status: "running".to_string(),
                    accepted: true,
                    message: "Input forwarded to host".to_string(),
                }),
            ))
        }
        Err(err) if err.contains("not found on any host") => {
            record_audit_event(
                &state,
                AuditEvent::new(
                    run.metadata.org_id.clone().unwrap_or_else(default_org_id),
                    "api",
                    "execution.input.ignored",
                    Some(run.id),
                    Some(run.task_id),
                    run.metadata.host_id.clone(),
                    run.metadata.trace_id.clone(),
                    Some("not_running".to_string()),
                    json!({
                        "reason": "not_found_on_host",
                        "source": "/api/v1/executions/{id}/input"
                    }),
                ),
            )
            .await;
            Ok((
                StatusCode::ACCEPTED,
                Json(ExecutionActionResponse {
                    execution_id: run.id,
                    task_id: run.task_id,
                    status: "not_running".to_string(),
                    accepted: false,
                    message: "Execution not active on any host; input ignored".to_string(),
                }),
            ))
        }
        Err(err) => Err(conflict_error(format!("Failed to send input: {}", err))),
    }
}

/// POST /api/v1/executions/:id/stop - Stop execution
async fn stop_execution(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(execution_id): Path<Uuid>,
) -> Result<(StatusCode, Json<ExecutionActionResponse>), RouteError> {
    let identity = resolve_user_identity(&headers, feature_multi_tenant()).map_err(unauthorized)?;
    let mut run = state
        .executor()
        .run_store()
        .find_run(execution_id)
        .map_err(internal_error)?
        .ok_or_else(|| not_found(format!("Execution {} not found", execution_id)))?;
    ensure_run_visible_to_identity(&run, identity.as_ref())?;

    if run.is_terminal() {
        let terminal_status = format!("{:?}", run.status).to_lowercase();
        record_audit_event(
            &state,
            AuditEvent::new(
                run.metadata.org_id.clone().unwrap_or_else(default_org_id),
                "api",
                "execution.stop.ignored",
                Some(run.id),
                Some(run.task_id),
                run.metadata.host_id.clone(),
                run.metadata.trace_id.clone(),
                Some(terminal_status.clone()),
                json!({ "reason": "terminal_state", "source": "/api/v1/executions/{id}/stop" }),
            ),
        )
        .await;
        return Ok((
            StatusCode::ACCEPTED,
            Json(ExecutionActionResponse {
                execution_id: run.id,
                task_id: run.task_id,
                status: terminal_status,
                accepted: false,
                message: "Execution already ended".to_string(),
            }),
        ));
    }

    match state
        .gateway_manager()
        .abort_task(&run.task_id.to_string())
        .await
    {
        Ok(()) => {
            run.mark_cancelled();
            state
                .executor()
                .run_store()
                .save_run(&run)
                .map_err(internal_error)?;
            record_audit_event(
                &state,
                AuditEvent::new(
                    run.metadata.org_id.clone().unwrap_or_else(default_org_id),
                    "api",
                    "execution.stop.accepted",
                    Some(run.id),
                    Some(run.task_id),
                    run.metadata.host_id.clone(),
                    run.metadata.trace_id.clone(),
                    Some("cancelled".to_string()),
                    json!({ "source": "/api/v1/executions/{id}/stop" }),
                ),
            )
            .await;
            Ok((
                StatusCode::ACCEPTED,
                Json(ExecutionActionResponse {
                    execution_id: run.id,
                    task_id: run.task_id,
                    status: "cancelled".to_string(),
                    accepted: true,
                    message: "Stop signal sent".to_string(),
                }),
            ))
        }
        Err(err) if err.contains("not found on any host") => {
            run.mark_cancelled();
            state
                .executor()
                .run_store()
                .save_run(&run)
                .map_err(internal_error)?;
            record_audit_event(
                &state,
                AuditEvent::new(
                    run.metadata.org_id.clone().unwrap_or_else(default_org_id),
                    "api",
                    "execution.stop.inferred",
                    Some(run.id),
                    Some(run.task_id),
                    run.metadata.host_id.clone(),
                    run.metadata.trace_id.clone(),
                    Some("cancelled".to_string()),
                    json!({
                        "reason": "not_found_on_host",
                        "source": "/api/v1/executions/{id}/stop"
                    }),
                ),
            )
            .await;
            Ok((
                StatusCode::ACCEPTED,
                Json(ExecutionActionResponse {
                    execution_id: run.id,
                    task_id: run.task_id,
                    status: "cancelled".to_string(),
                    accepted: false,
                    message: "Execution already stopped".to_string(),
                }),
            ))
        }
        Err(err) => Err(conflict_error(format!("Failed to stop execution: {}", err))),
    }
}

/// GET /api/v1/executions/:id/events - Query execution event stream
async fn list_execution_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(execution_id): Path<Uuid>,
    Query(query): Query<ExecutionEventsQuery>,
) -> Result<Json<ExecutionEventsResponse>, RouteError> {
    let identity = resolve_user_identity(&headers, feature_multi_tenant()).map_err(unauthorized)?;
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(200).min(1000);

    let loaded = state
        .executor()
        .run_store()
        .load_events_by_run_id_filtered_paginated(
            execution_id,
            offset,
            limit,
            query.event_type.as_deref(),
            query.agent_event_type.as_deref(),
        )
        .map_err(internal_error)?;

    let Some((run, events, has_more)) = loaded else {
        return Err(not_found(format!("Execution {} not found", execution_id)));
    };
    ensure_run_visible_to_identity(&run, identity.as_ref())?;

    let org_id = run.metadata.org_id.clone().unwrap_or_else(default_org_id);
    let trace_id = run
        .metadata
        .trace_id
        .clone()
        .unwrap_or_else(|| run.id.to_string());
    let host_id = run.metadata.host_id.clone();

    let events = events
        .into_iter()
        .enumerate()
        .map(|(index, payload)| ExecutionEventEnvelope {
            execution_id: run.id,
            org_id: org_id.clone(),
            trace_id: trace_id.clone(),
            seq: (offset + index + 1) as u64,
            ts: payload.timestamp.timestamp_millis(),
            task_id: run.task_id,
            host_id: host_id.clone(),
            payload,
        })
        .collect::<Vec<_>>();

    let next_offset = if has_more {
        Some(offset + events.len())
    } else {
        None
    };

    Ok(Json(ExecutionEventsResponse {
        events,
        has_more,
        next_offset,
    }))
}

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/executions",
            get(list_executions).post(create_execution),
        )
        .route("/api/v1/executions/{id}", get(get_execution))
        .route("/api/v1/executions/{id}/input", post(input_execution))
        .route("/api/v1/executions/{id}/stop", post(stop_execution))
        .route("/api/v1/executions/{id}/events", get(list_execution_events))
}

// ============================================================================
// Helpers
// ============================================================================

fn internal_error(error: impl std::fmt::Display) -> RouteError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
}

fn not_found(error: impl Into<String>) -> RouteError {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: error.into(),
        }),
    )
}

fn conflict_error(error: impl Into<String>) -> RouteError {
    (
        StatusCode::CONFLICT,
        Json(ErrorResponse {
            error: error.into(),
        }),
    )
}

fn unauthorized(error: impl Into<String>) -> RouteError {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: error.into(),
        }),
    )
}

fn default_org_id() -> String {
    std::env::var("VK_DEFAULT_ORG_ID")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "default-org".to_string())
}

fn ensure_run_visible_to_identity(
    run: &Run,
    identity: Option<&UserIdentity>,
) -> Result<(), RouteError> {
    let Some(identity) = identity else {
        return Ok(());
    };
    let run_org_id = run.metadata.org_id.clone().unwrap_or_else(default_org_id);
    if run_org_id != identity.org_id {
        return Err(not_found(format!("Execution {} not found", run.id)));
    }
    Ok(())
}

async fn record_audit_event(state: &AppState, event: AuditEvent) {
    if let Err(err) = state.audit_store().append(event).await {
        warn!("Failed to append execution audit event: {}", err);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{collections::HashMap, sync::Arc};

    use agent_runner::{AgentType, ExecutionEvent};
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use serde_json::{json, Value};
    use tempfile::TempDir;
    use tower::ServiceExt;
    use vk_core::{
        kanban::KanbanStore,
        project::CreateProjectRequest,
        task::{FileTaskStore, Task, TaskRepository},
    };

    use crate::{
        gateway::{protocol::HostCapabilities, GatewayManager},
        state::AppState,
    };

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
    async fn create_execution_returns_execution_identity() {
        let (state, _temp_dir) = build_state().await;
        let workspace_id = state.workspace_store().list().await[0].id;
        let project = state
            .project_store()
            .register(
                "host-e3".to_string(),
                CreateProjectRequest {
                    name: "execution-project".to_string(),
                    local_path: "/tmp/execution-project".to_string(),
                    remote_url: None,
                    default_branch: Some("main".to_string()),
                    worktree_dir: None,
                    workspace_id,
                    org_id: None,
                },
            )
            .await
            .unwrap();
        let task = state
            .task_store()
            .create(Task::new("execution task".to_string()).with_project_id(project.id))
            .await
            .unwrap();

        let (tx, _rx) = tokio::sync::mpsc::channel(10);
        state
            .gateway_manager()
            .register_host(
                project.gateway_id.clone(),
                HostCapabilities {
                    name: "E3 host".to_string(),
                    agents: vec!["opencode".to_string()],
                    max_concurrent: 2,
                    cwd: "/tmp".to_string(),
                    labels: HashMap::new(),
                },
                tx,
            )
            .await;

        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/executions")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "taskId": task.id,
                            "traceId": "trace-e3",
                            "orgId": project.org_id,
                            "agentType": "opencode"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["taskId"], task.id.to_string());
        assert_eq!(payload["traceId"], "trace-e3");
        assert_eq!(payload["orgId"], project.org_id);
    }

    #[tokio::test]
    async fn get_execution_returns_run_details() {
        let (state, _temp_dir) = build_state().await;
        let task = state
            .task_store()
            .create(Task::new("details task".to_string()))
            .await
            .unwrap();

        let mut run = Run::new(
            task.id,
            AgentType::OpenCode,
            "prompt".to_string(),
            "main".to_string(),
        );
        run.metadata.trace_id = Some("trace-details".to_string());
        run.metadata.org_id = Some("org-details".to_string());
        state.executor().run_store().save_run(&run).unwrap();

        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/executions/{}", run.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["executionId"], run.id.to_string());
        assert_eq!(payload["traceId"], "trace-details");
        assert_eq!(payload["orgId"], "org-details");
    }

    #[tokio::test]
    async fn list_execution_events_wraps_with_orchestrator_envelope() {
        let (state, _temp_dir) = build_state().await;
        let task = state
            .task_store()
            .create(Task::new("events task".to_string()))
            .await
            .unwrap();

        let mut run = Run::new(
            task.id,
            AgentType::OpenCode,
            "prompt".to_string(),
            "main".to_string(),
        );
        run.metadata.trace_id = Some("trace-events".to_string());
        run.metadata.org_id = Some("org-events".to_string());
        state.executor().run_store().save_run(&run).unwrap();

        let event = ExecutionEvent::progress(run.id, task.id, "running".to_string(), Some(0.5));
        state
            .executor()
            .run_store()
            .append_event(task.id, run.id, &event)
            .unwrap();

        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/executions/{}/events", run.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let first_event = payload["events"].as_array().unwrap().first().unwrap();

        assert_eq!(first_event["executionId"], run.id.to_string());
        assert_eq!(first_event["orgId"], "org-events");
        assert_eq!(first_event["traceId"], "trace-events");
        assert_eq!(first_event["seq"], 1);
        assert!(first_event["ts"].is_i64());
        assert!(first_event["payload"].is_object());
    }

    #[tokio::test]
    async fn list_executions_supports_filters() {
        let (state, _temp_dir) = build_state().await;
        let task = state
            .task_store()
            .create(Task::new("list task".to_string()))
            .await
            .unwrap();

        let mut run = Run::new(
            task.id,
            AgentType::OpenCode,
            "prompt".to_string(),
            "main".to_string(),
        );
        run.metadata.org_id = Some("org-list".to_string());
        run.metadata.host_id = Some("host-list".to_string());
        run.mark_started();
        state.executor().run_store().save_run(&run).unwrap();

        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/executions?orgId=org-list&hostId=host-list&status=running")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let items = payload["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["executionId"], run.id.to_string());
    }
}
