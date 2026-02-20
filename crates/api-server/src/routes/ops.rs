//! Ops console aggregation APIs.
//!
//! These endpoints provide dashboard-focused views for hosts, executions,
//! and audit logs without changing core task/execution semantics.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use agent_runner::Run;
use vk_core::task::TaskRepository;

use crate::{
    audit::{AuditListQuery, AuditListResponse},
    gateway::protocol::HostConnectionStatus,
    state::AppState,
};

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

type RouteError = (StatusCode, Json<ErrorResponse>);

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct OpsExecutionsQuery {
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    task_id: Option<Uuid>,
    #[serde(default)]
    project_id: Option<Uuid>,
    #[serde(default)]
    workspace_id: Option<Uuid>,
    #[serde(default)]
    host_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HostSummary {
    total: u32,
    online: u32,
    busy: u32,
    offline: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExecutionSummary {
    total: u32,
    running: u32,
    completed: u32,
    failed: u32,
    cancelled: u32,
    other: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemorySummary {
    enabled: bool,
    gateway_store_enabled: bool,
    rust_store_enabled: bool,
    token_budget: u32,
    retrieval_top_k: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditSummary {
    total: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpsSummaryResponse {
    updated_at: String,
    hosts: HostSummary,
    executions: ExecutionSummary,
    memory: MemorySummary,
    audit: AuditSummary,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpsExecutionItem {
    execution_id: Uuid,
    task_id: Uuid,
    project_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
    host_id: Option<String>,
    trace_id: Option<String>,
    org_id: Option<String>,
    parent_execution_id: Option<Uuid>,
    agent_role: Option<String>,
    handoff_id: Option<String>,
    agent_type: String,
    base_branch: String,
    status: String,
    created_at: String,
    started_at: Option<String>,
    ended_at: Option<String>,
    duration_ms: Option<u64>,
    summary: Option<String>,
    error: Option<String>,
    event_count: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpsExecutionsResponse {
    items: Vec<OpsExecutionItem>,
    has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_offset: Option<usize>,
}

impl OpsExecutionItem {
    fn from_run(run: &Run, project_id: Option<Uuid>, workspace_id: Option<Uuid>, host_id: Option<String>) -> Self {
        Self {
            execution_id: run.id,
            task_id: run.task_id,
            project_id,
            workspace_id,
            host_id,
            trace_id: None,
            org_id: None,
            parent_execution_id: None,
            agent_role: None,
            handoff_id: None,
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

async fn resolve_run_context(
    state: &AppState,
    run: &Run,
) -> (Option<Uuid>, Option<Uuid>, Option<String>) {
    let mut project_id = run.metadata.project_id;
    let mut workspace_id = run.metadata.workspace_id;
    let mut host_id = None;

    if project_id.is_none() || workspace_id.is_none() {
        if let Ok(Some(task)) = state.task_store().get(run.task_id).await {
            if project_id.is_none() {
                project_id = task.project_id;
            }
            if workspace_id.is_none() {
                workspace_id = task.workspace_id;
            }
        }
    }

    if let Some(project_id_value) = project_id {
        if let Some(project) = state.project_store().get(project_id_value).await {
            if workspace_id.is_none() {
                workspace_id = Some(project.workspace_id);
            }
            host_id = Some(project.gateway_id);
        }
    }

    (project_id, workspace_id, host_id)
}

async fn get_ops_summary(State(state): State<AppState>) -> Result<Json<OpsSummaryResponse>, RouteError> {
    let hosts = state.gateway_manager().list_hosts().await;
    let mut host_summary = HostSummary {
        total: 0,
        online: 0,
        busy: 0,
        offline: 0,
    };

    for host in hosts {
        host_summary.total = host_summary.total.saturating_add(1);
        match host.status {
            HostConnectionStatus::Online => {
                host_summary.online = host_summary.online.saturating_add(1);
            }
            HostConnectionStatus::Busy => {
                host_summary.busy = host_summary.busy.saturating_add(1);
            }
            HostConnectionStatus::Offline => {
                host_summary.offline = host_summary.offline.saturating_add(1);
            }
        }
    }

    let (runs, _) = state
        .executor()
        .run_store()
        .list_runs_global_paginated(0, 5000, None, None)
        .map_err(internal_error)?;

    let mut execution_summary = ExecutionSummary {
        total: runs.len() as u32,
        running: 0,
        completed: 0,
        failed: 0,
        cancelled: 0,
        other: 0,
    };

    for run in runs {
        let status = format!("{:?}", run.status).to_lowercase();
        match status.as_str() {
            "running" | "initializing" | "starting" | "creatingworktree" | "paused"
            | "cleaningup" => {
                execution_summary.running = execution_summary.running.saturating_add(1);
            }
            "completed" => {
                execution_summary.completed = execution_summary.completed.saturating_add(1);
            }
            "failed" => {
                execution_summary.failed = execution_summary.failed.saturating_add(1);
            }
            "cancelled" => {
                execution_summary.cancelled = execution_summary.cancelled.saturating_add(1);
            }
            _ => execution_summary.other = execution_summary.other.saturating_add(1),
        }
    }

    let settings = state.memory_store().get_settings().await;
    let memory = MemorySummary {
        enabled: settings.enabled,
        gateway_store_enabled: settings.gateway_store_enabled,
        rust_store_enabled: settings.rust_store_enabled,
        token_budget: settings.token_budget,
        retrieval_top_k: settings.retrieval_top_k,
    };

    let audit = AuditSummary {
        total: state.audit_store().count().await as u32,
    };

    Ok(Json(OpsSummaryResponse {
        updated_at: Utc::now().to_rfc3339(),
        hosts: host_summary,
        executions: execution_summary,
        memory,
        audit,
    }))
}

async fn list_ops_executions(
    State(state): State<AppState>,
    Query(query): Query<OpsExecutionsQuery>,
) -> Result<Json<OpsExecutionsResponse>, RouteError> {
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let host_filter = query
        .host_id
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());

    let (runs, _) = state
        .executor()
        .run_store()
        .list_runs_global_paginated(0, 5000, query.status.as_deref(), query.task_id)
        .map_err(internal_error)?;

    let mut all_items = Vec::new();
    for run in runs {
        let (project_id, workspace_id, host_id) = resolve_run_context(&state, &run).await;

        if query
            .project_id
            .is_some_and(|expected_project_id| project_id != Some(expected_project_id))
        {
            continue;
        }

        if query
            .workspace_id
            .is_some_and(|expected_workspace_id| workspace_id != Some(expected_workspace_id))
        {
            continue;
        }

        if let Some(host_filter) = host_filter.as_deref() {
            let current_host = host_id.as_deref().unwrap_or("").to_ascii_lowercase();
            if current_host != host_filter {
                continue;
            }
        }

        all_items.push(OpsExecutionItem::from_run(
            &run,
            project_id,
            workspace_id,
            host_id,
        ));
    }

    let total = all_items.len();
    let items = all_items
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let has_more = total > offset + items.len();
    let next_offset = if has_more {
        Some(offset + items.len())
    } else {
        None
    };

    Ok(Json(OpsExecutionsResponse {
        items,
        has_more,
        next_offset,
    }))
}

async fn list_ops_audit(
    State(state): State<AppState>,
    Query(query): Query<AuditListQuery>,
) -> Result<Json<AuditListResponse>, RouteError> {
    let (items, has_more) = state.audit_store().list_paginated(&query).await;
    let offset = query.offset.unwrap_or(0);
    let next_offset = if has_more {
        Some(offset + items.len())
    } else {
        None
    };

    Ok(Json(AuditListResponse {
        items,
        has_more,
        next_offset,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/ops/summary", get(get_ops_summary))
        .route("/api/v1/ops/executions", get(list_ops_executions))
        .route("/api/v1/ops/audit", get(list_ops_audit))
}

fn internal_error(error: impl std::fmt::Display) -> RouteError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use agent_runner::{AgentType, Run};
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use serde_json::Value;
    use tempfile::TempDir;
    use tower::ServiceExt;
    use vk_core::{
        kanban::KanbanStore,
        task::{FileTaskStore, Task, TaskRepository},
    };

    use crate::{
        audit::AuditEvent,
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
    async fn summary_endpoint_returns_counts() {
        let (state, _tmp) = build_state().await;
        let task = state
            .task_store()
            .create(Task::new("ops summary task".to_string()))
            .await
            .unwrap();

        let mut run = Run::new(
            task.id,
            AgentType::OpenCode,
            "prompt".to_string(),
            "main".to_string(),
        );
        run.mark_started();
        state.executor().run_store().save_run(&run).unwrap();

        let (tx, _rx) = tokio::sync::mpsc::channel(4);
        state
            .gateway_manager()
            .register_host(
                "ops-host".to_string(),
                HostCapabilities {
                    name: "ops-host".to_string(),
                    agents: vec!["opencode".to_string()],
                    max_concurrent: 2,
                    cwd: "/tmp".to_string(),
                    labels: HashMap::new(),
                },
                tx,
            )
            .await;

        let app = super::router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/ops/summary")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["hosts"]["total"], 1);
        assert_eq!(payload["executions"]["running"], 1);
    }

    #[tokio::test]
    async fn audit_endpoint_returns_latest_first() {
        let (state, _tmp) = build_state().await;
        state
            .audit_store()
            .append(AuditEvent::new(
                "default",
                "api",
                "execution.created",
                None,
                None,
                None,
                Some("accepted".to_string()),
                serde_json::Value::Null,
            ))
            .await
            .unwrap();

        let app = super::router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/ops/audit")
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
        assert_eq!(items[0]["action"], "execution.created");
    }
}
