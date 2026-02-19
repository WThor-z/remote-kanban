//! Ops console aggregation APIs
//!
//! These endpoints are tailored for operations dashboards and do not
//! change task/execution core semantics.

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use agent_runner::Run;

use crate::{
    audit::{AuditListQuery, AuditListResponse},
    auth::resolve_user_identity,
    feature_flags::feature_multi_tenant,
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
    org_id: Option<String>,
    #[serde(default)]
    host_id: Option<String>,
    #[serde(default)]
    task_id: Option<Uuid>,
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
struct OpsSummaryResponse {
    updated_at: String,
    hosts: HostSummary,
    executions: ExecutionSummary,
    memory: MemorySummary,
}

impl From<&Run> for OpsExecutionItem {
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

async fn get_ops_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OpsSummaryResponse>, RouteError> {
    let identity = resolve_user_identity(&headers, feature_multi_tenant()).map_err(unauthorized)?;
    let allowed_host_ids = if let Some(identity) = identity.as_ref() {
        let hosts = state
            .auth_store()
            .list_hosts_for_org(&identity.org_id)
            .await;
        Some(
            hosts
                .into_iter()
                .map(|host| host.host_id)
                .collect::<std::collections::HashSet<_>>(),
        )
    } else {
        None
    };

    let hosts = state.gateway_manager().list_hosts().await;
    let mut host_summary = HostSummary {
        total: 0,
        online: 0,
        busy: 0,
        offline: 0,
    };

    for host in hosts {
        if let Some(allowed) = allowed_host_ids.as_ref() {
            if !allowed.contains(&host.host_id) {
                continue;
            }
        }
        host_summary.total = host_summary.total.saturating_add(1);
        match host.status {
            HostConnectionStatus::Online => {
                host_summary.online = host_summary.online.saturating_add(1)
            }
            HostConnectionStatus::Busy => host_summary.busy = host_summary.busy.saturating_add(1),
            HostConnectionStatus::Offline => {
                host_summary.offline = host_summary.offline.saturating_add(1)
            }
        }
    }

    let (runs, _) = state
        .executor()
        .run_store()
        .list_runs_global_filtered_paginated(
            0,
            5000,
            None,
            identity.as_ref().map(|current| current.org_id.as_str()),
            None,
            None,
        )
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
            "running" | "initializing" | "starting" | "creating_worktree" | "paused"
            | "cleaning_up" => {
                execution_summary.running = execution_summary.running.saturating_add(1)
            }
            "completed" => {
                execution_summary.completed = execution_summary.completed.saturating_add(1)
            }
            "failed" => execution_summary.failed = execution_summary.failed.saturating_add(1),
            "cancelled" => {
                execution_summary.cancelled = execution_summary.cancelled.saturating_add(1)
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

    Ok(Json(OpsSummaryResponse {
        updated_at: Utc::now().to_rfc3339(),
        hosts: host_summary,
        executions: execution_summary,
        memory,
    }))
}

async fn list_ops_executions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<OpsExecutionsQuery>,
) -> Result<Json<OpsExecutionsResponse>, RouteError> {
    let identity = resolve_user_identity(&headers, feature_multi_tenant()).map_err(unauthorized)?;
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let org_filter = identity
        .as_ref()
        .map(|current| current.org_id.as_str())
        .or(query.org_id.as_deref());

    let (runs, has_more) = state
        .executor()
        .run_store()
        .list_runs_global_filtered_paginated(
            offset,
            limit,
            query.status.as_deref(),
            org_filter,
            query.host_id.as_deref(),
            query.task_id,
        )
        .map_err(internal_error)?;

    let items = runs.iter().map(OpsExecutionItem::from).collect::<Vec<_>>();
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
    headers: HeaderMap,
    Query(query): Query<AuditListQuery>,
) -> Result<Json<AuditListResponse>, RouteError> {
    let identity = resolve_user_identity(&headers, feature_multi_tenant()).map_err(unauthorized)?;
    let mut scoped_query = query;
    if let Some(identity) = identity.as_ref() {
        scoped_query.org_id = Some(identity.org_id.clone());
    }

    let (items, has_more) = state.audit_store().list_paginated(&scoped_query).await;
    let offset = scoped_query.offset.unwrap_or(0);
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

fn unauthorized(error: impl Into<String>) -> RouteError {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: error.into(),
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
                "org-ops".to_string(),
                "api",
                "execution.created",
                None,
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
        assert_eq!(items[0]["orgId"], "org-ops");
        assert_eq!(items[0]["action"], "execution.created");
    }
}
