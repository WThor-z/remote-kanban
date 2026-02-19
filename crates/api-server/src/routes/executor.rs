//! Executor API endpoints
//!
//! RESTful API for task execution operations.

use axum::{
    extract::{Path, State},
    http::{header::HeaderName, HeaderMap, HeaderValue, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tower_http::set_header::SetResponseHeaderLayer;
use uuid::Uuid;

use agent_runner::{
    AgentType, ChatMessage, ExecutionEvent, ExecutionStatus, MessageRole, Run, SessionState,
};
use vk_core::kanban::KanbanTaskStatus;
use vk_core::task::TaskRepository;

use crate::audit::AuditEvent;
use crate::auth::{resolve_user_identity, UserIdentity};
use crate::feature_flags::feature_multi_tenant;
use crate::gateway::protocol::{GatewayAgentEvent, GatewayAgentEventType, GatewayTaskRequest};
use crate::state::AppState;

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartExecutionRequest {
    pub agent_type: String,
    pub base_branch: String,
    /// Optional target host for remote execution
    pub target_host: Option<String>,
    /// Optional model to use (format: provider/model)
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendInputRequest {
    pub content: String,
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
pub(crate) struct ErrorResponse {
    pub error: String,
}

pub(crate) type RouteError = (StatusCode, Json<ErrorResponse>);

#[derive(Debug, Clone)]
pub(crate) struct StartTaskExecutionCommand {
    pub task_id: Uuid,
    pub agent_type: String,
    pub base_branch: Option<String>,
    pub target_host: Option<String>,
    pub model: Option<String>,
    pub trace_id: Option<String>,
    pub org_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct StartTaskExecutionResult {
    pub execution_id: Uuid,
    pub task_id: Uuid,
    pub status: String,
    pub message: String,
    pub host_id: String,
    pub trace_id: String,
    pub org_id: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /api/tasks/:id/execute - Start task execution
async fn start_execution(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<Uuid>,
    Json(req): Json<StartExecutionRequest>,
) -> Result<(StatusCode, Json<ExecutionResponse>), RouteError> {
    let identity = resolve_user_identity(&headers, feature_multi_tenant()).map_err(unauthorized)?;
    tracing::info!(
        "Execute request for task {}: agent_type={}, target_host={:?}, model={:?}",
        task_id,
        req.agent_type,
        req.target_host,
        req.model
    );

    let result = start_task_execution(
        &state,
        StartTaskExecutionCommand {
            task_id,
            agent_type: req.agent_type,
            base_branch: Some(req.base_branch),
            target_host: req.target_host,
            model: req.model,
            trace_id: None,
            org_id: identity.as_ref().map(|current| current.org_id.clone()),
        },
    )
    .await?;

    record_audit_event(
        &state,
        AuditEvent::new(
            result.org_id.clone(),
            "api",
            "task.execute.legacy",
            Some(result.execution_id),
            Some(result.task_id),
            Some(result.host_id.clone()),
            Some(result.trace_id.clone()),
            Some(result.status.clone()),
            json!({
                "source": "/api/tasks/{id}/execute",
                "deprecated": true
            }),
        ),
    )
    .await;

    Ok((
        StatusCode::ACCEPTED,
        Json(ExecutionResponse {
            session_id: result.execution_id,
            task_id: result.task_id,
            status: result.status,
            message: result.message,
        }),
    ))
}

pub(crate) async fn start_task_execution(
    state: &AppState,
    req: StartTaskExecutionCommand,
) -> Result<StartTaskExecutionResult, RouteError> {
    let task_id = req.task_id;
    let mut task = state
        .task_store()
        .get(task_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found(format!("Task {} not found", task_id)))?;

    let project_id = task
        .project_id
        .ok_or_else(|| validation_error("Project is required"))?;
    let project = state
        .project_store()
        .get(project_id)
        .await
        .ok_or_else(|| not_found(format!("Project {} not found", project_id)))?;
    let project_org_id = normalize_org_id(&project.org_id);

    if task.org_id.trim().is_empty() {
        task.org_id = project_org_id.clone();
        task = state
            .task_store()
            .update(task)
            .await
            .map_err(internal_error)?;
    } else if task.org_id != project_org_id {
        return Err(conflict_error(format!(
            "Task org {} does not match project org {}",
            task.org_id, project_org_id
        )));
    }

    match task.workspace_id {
        Some(workspace_id) if workspace_id != project.workspace_id => {
            return Err(conflict_error(format!(
                "Task workspace {} does not match project workspace {}",
                workspace_id, project.workspace_id
            )));
        }
        Some(_) => {}
        None => {
            task.workspace_id = Some(project.workspace_id);
            task = state
                .task_store()
                .update(task)
                .await
                .map_err(internal_error)?;
        }
    }

    if let Some(target_host) = req.target_host.as_deref() {
        if target_host != project.gateway_id {
            return Err(conflict_error(format!(
                "Task is bound to host {} and cannot run on {}",
                project.gateway_id, target_host
            )));
        }
    }

    let prompt = if let Some(desc) = &task.description {
        format!("{}\n\n{}", task.title, desc)
    } else {
        task.title.clone()
    };

    let base_branch = req
        .base_branch
        .filter(|branch| !branch.trim().is_empty())
        .or_else(|| task.base_branch.clone())
        .unwrap_or_else(|| project.default_branch.clone());
    let agent_type = if req.agent_type.trim().is_empty() {
        task.agent_type
            .clone()
            .unwrap_or_else(|| "opencode".to_string())
    } else {
        req.agent_type
    };

    let trace_id = req
        .trace_id
        .filter(|trace| !trace.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    if let Some(requested_org_id) = req
        .org_id
        .as_deref()
        .map(str::trim)
        .filter(|org| !org.is_empty())
    {
        if requested_org_id != project_org_id {
            return Err(conflict_error(
                "Requested org does not match task/project organization",
            ));
        }
    }
    let org_id = project_org_id;

    dispatch_to_gateway(
        state,
        task_id,
        &prompt,
        &agent_type,
        &project.gateway_id,
        &task.title,
        task.description.as_deref(),
        &project.local_path,
        req.model.as_deref(),
        &base_branch,
        project.id,
        project.workspace_id,
        &trace_id,
        &org_id,
    )
    .await
}

/// Dispatch task to a remote Gateway host
async fn dispatch_to_gateway(
    state: &AppState,
    task_id: Uuid,
    prompt: &str,
    agent_type: &str,
    target_host: &str,
    task_title: &str,
    task_description: Option<&str>,
    cwd: &str,
    model: Option<&str>,
    base_branch: &str,
    project_id: Uuid,
    workspace_id: Uuid,
    trace_id: &str,
    org_id: &str,
) -> Result<StartTaskExecutionResult, RouteError> {
    let gateway_manager = state.gateway_manager();
    let memory_settings_snapshot =
        serde_json::to_value(state.memory_store().get_settings().await).ok();

    let gateway_task = GatewayTaskRequest {
        task_id: task_id.to_string(),
        prompt: prompt.to_string(),
        cwd: cwd.to_string(),
        agent_type: agent_type.to_string(),
        model: model.map(String::from),
        env: HashMap::new(),
        timeout: None,
        metadata: serde_json::json!({
            "projectId": project_id,
            "workspaceId": workspace_id,
            "taskId": task_id,
            "taskTitle": task_title,
            "taskDescription": task_description,
            "traceId": trace_id,
            "orgId": org_id,
            "memorySettingsSnapshot": memory_settings_snapshot,
        }),
    };

    match gateway_manager
        .dispatch_task_to_host(target_host, gateway_task)
        .await
    {
        Ok(host_id) => {
            tracing::info!("Task {} dispatched to gateway host {}", task_id, host_id);

            // Create a Run record for this gateway execution
            let run_id = Uuid::new_v4();
            let parsed_agent_type = AgentType::from_str(agent_type).unwrap_or(AgentType::OpenCode);
            let mut run = Run::new(
                task_id,
                parsed_agent_type,
                prompt.to_string(),
                base_branch.to_string(),
            );
            // Override the generated ID to use our run_id
            run.id = run_id;
            run.metadata.project_id = Some(project_id);
            run.metadata.workspace_id = Some(workspace_id);
            run.metadata.trace_id = Some(trace_id.to_string());
            run.metadata.org_id = Some(org_id.to_string());
            run.metadata.host_id = Some(host_id.clone());
            run.mark_started();

            // Save the initial run record
            if let Err(e) = state.executor().run_store().save_run(&run) {
                tracing::warn!(
                    "Failed to save initial run record for gateway task {}: {}",
                    task_id,
                    e
                );
            } else {
                tracing::info!("Created run {} for gateway task {}", run_id, task_id);
            }

            // Set up event forwarding from Gateway to Socket.IO
            let state_clone = state.clone();
            let task_id_str = task_id.to_string();
            let prompt_clone = prompt.to_string();
            let agent_type_clone = parsed_agent_type;
            let base_branch = base_branch.to_string();
            let project_id = project_id;
            let workspace_id = workspace_id;
            let host_id_clone = host_id.clone();
            let trace_id_for_response = trace_id.to_string();
            let org_id_for_response = org_id.to_string();
            let trace_id_for_events = trace_id_for_response.clone();
            let org_id_for_events = org_id_for_response.clone();
            tokio::spawn(async move {
                let mut event_rx = state_clone.gateway_manager().subscribe();
                let io = state_clone.get_socket_io().await;
                let mut event_count: u32 = 0;
                // Accumulate stdout content for final message
                let mut accumulated_output = String::new();
                // Fixed message ID for streaming updates
                let message_id = uuid::Uuid::new_v4().to_string();

                let started_event = execution_event_from_gateway(
                    run_id,
                    task_id,
                    &GatewayAgentEvent {
                        event_type: GatewayAgentEventType::Log,
                        content: Some(format!(
                            "Execution dispatched to host {} (trace: {})",
                            host_id_clone, trace_id_for_events
                        )),
                        data: serde_json::json!({}),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
                    },
                );
                if let Err(err) =
                    state_clone
                        .executor()
                        .run_store()
                        .append_event(task_id, run_id, &started_event)
                {
                    tracing::warn!(
                        "Failed to persist start event for task {} run {}: {}",
                        task_id_str,
                        run_id,
                        err
                    );
                } else {
                    event_count = event_count.saturating_add(1);
                }

                if let Some(io) = io {
                    // Move task to Doing when execution starts
                    {
                        let kanban_store = state_clone.kanban_store();
                        if let Err(e) = kanban_store
                            .move_task(&task_id_str, KanbanTaskStatus::Doing, None)
                            .await
                        {
                            tracing::warn!(
                                "Failed to move kanban task {} to Doing: {}",
                                task_id_str,
                                e
                            );
                        } else {
                            tracing::info!("Kanban task {} moved to Doing", task_id_str);
                            // Broadcast initial kanban sync
                            let board_state = kanban_store.get_state().await;
                            let _ = io.emit("kanban:sync", &board_state);
                        }
                    }

                    // Send initial "working" message
                    {
                        #[derive(serde::Serialize)]
                        struct MessagePayload {
                            #[serde(rename = "taskId")]
                            task_id: String,
                            message: TaskMessage,
                        }
                        #[derive(serde::Serialize)]
                        struct TaskMessage {
                            id: String,
                            role: String,
                            content: String,
                            timestamp: i64,
                            #[serde(rename = "isStreaming")]
                            is_streaming: bool,
                        }

                        let _ = io.emit(
                            "task:message",
                            &MessagePayload {
                                task_id: task_id_str.clone(),
                                message: TaskMessage {
                                    id: message_id.clone(),
                                    role: "assistant".to_string(),
                                    content: "姝ｅ湪宸ヤ綔涓?..".to_string(),
                                    timestamp: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis()
                                        as i64,
                                    is_streaming: true,
                                },
                            },
                        );
                    }

                    while let Ok(event) = event_rx.recv().await {
                        if event.task_id == task_id_str {
                            let execution_event =
                                execution_event_from_gateway(run_id, task_id, &event.event);
                            if let Err(err) = state_clone.executor().run_store().append_event(
                                task_id,
                                run_id,
                                &execution_event,
                            ) {
                                tracing::warn!(
                                    "Failed to persist gateway event for task {} run {}: {}",
                                    task_id_str,
                                    run_id,
                                    err
                                );
                            } else {
                                event_count = event_count.saturating_add(1);
                            }

                            // Forward gateway event to Socket.IO for Logs panel
                            let _ = io.emit("task:gateway_event", &event);

                            // Accumulate stdout content
                            if let Some(content) = &event.event.content {
                                match event.event.event_type {
                                    GatewayAgentEventType::Stdout => {
                                        if !content.starts_with("[executor]")
                                            && !content.starts_with("[Gateway]")
                                            && !content.is_empty()
                                        {
                                            accumulated_output.push_str(content);
                                        }
                                    }
                                    GatewayAgentEventType::Message => {
                                        accumulated_output.push_str(content);
                                    }
                                    _ => {}
                                }
                            }

                            // Check for Completed/Failed events and update Run record
                            match event.event.event_type {
                                GatewayAgentEventType::Completed => {
                                    // Update Run record
                                    let mut run = Run::new(
                                        task_id,
                                        agent_type_clone,
                                        prompt_clone.clone(),
                                        base_branch.clone(),
                                    );
                                    run.id = run_id;
                                    run.metadata.project_id = Some(project_id);
                                    run.metadata.workspace_id = Some(workspace_id);
                                    run.metadata.trace_id = Some(trace_id_for_events.clone());
                                    run.metadata.org_id = Some(org_id_for_events.clone());
                                    run.metadata.host_id = Some(host_id_clone.clone());
                                    run.mark_started();
                                    run.mark_completed(0, event.event.content.clone());
                                    run.event_count = event_count;

                                    let session_end_event = execution_session_end_event(
                                        run_id,
                                        task_id,
                                        true,
                                        event.event.timestamp,
                                        run.started_at.unwrap_or(run.created_at),
                                    );
                                    if let Err(err) = state_clone
                                        .executor()
                                        .run_store()
                                        .append_event(task_id, run_id, &session_end_event)
                                    {
                                        tracing::warn!(
                                            "Failed to persist completion session event for task {} run {}: {}",
                                            task_id_str,
                                            run_id,
                                            err
                                        );
                                    } else {
                                        event_count = event_count.saturating_add(1);
                                        run.event_count = event_count;
                                    }

                                    if let Err(e) =
                                        state_clone.executor().run_store().save_run(&run)
                                    {
                                        tracing::warn!(
                                            "Failed to save completed run for task {}: {}",
                                            task_id_str,
                                            e
                                        );
                                    } else {
                                        tracing::info!(
                                            "Run {} completed for gateway task {}",
                                            run_id,
                                            task_id_str
                                        );
                                    }

                                    // Send final complete message (replacing the streaming one)
                                    {
                                        #[derive(serde::Serialize)]
                                        struct MessagePayload {
                                            #[serde(rename = "taskId")]
                                            task_id: String,
                                            message: TaskMessage,
                                        }
                                        #[derive(serde::Serialize)]
                                        struct TaskMessage {
                                            id: String,
                                            role: String,
                                            content: String,
                                            timestamp: i64,
                                            #[serde(rename = "isStreaming")]
                                            is_streaming: bool,
                                        }

                                        let final_content = if accumulated_output.is_empty() {
                                            event
                                                .event
                                                .content
                                                .clone()
                                                .unwrap_or_else(|| "浠诲姟瀹屾垚".to_string())
                                        } else {
                                            accumulated_output.clone()
                                        };

                                        let msg_timestamp = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_millis()
                                            as i64;

                                        let _ = io.emit(
                                            "task:message",
                                            &MessagePayload {
                                                task_id: task_id_str.clone(),
                                                message: TaskMessage {
                                                    id: message_id.clone(),
                                                    role: "assistant".to_string(),
                                                    content: final_content.clone(),
                                                    timestamp: msg_timestamp,
                                                    is_streaming: false,
                                                },
                                            },
                                        );

                                        // Persist the message to disk
                                        let chat_msg = ChatMessage::with_id(
                                            message_id.clone(),
                                            MessageRole::Assistant,
                                            final_content,
                                        );
                                        if let Err(e) = state_clone
                                            .executor()
                                            .run_store()
                                            .append_message(task_id, run_id, &chat_msg)
                                        {
                                            tracing::warn!(
                                                "Failed to persist message for task {}: {}",
                                                task_id_str,
                                                e
                                            );
                                        }
                                    }

                                    // Broadcast kanban sync
                                    let kanban_store = state_clone.kanban_store();
                                    let board_state = kanban_store.get_state().await;
                                    let _ = io.emit("kanban:sync", &board_state);
                                    tracing::info!(
                                        "Broadcasted kanban:sync after task {} completed",
                                        task_id_str
                                    );
                                    break;
                                }
                                GatewayAgentEventType::Failed => {
                                    // Update Run record
                                    let mut run = Run::new(
                                        task_id,
                                        agent_type_clone,
                                        prompt_clone.clone(),
                                        base_branch.clone(),
                                    );
                                    run.id = run_id;
                                    run.metadata.project_id = Some(project_id);
                                    run.metadata.workspace_id = Some(workspace_id);
                                    run.metadata.trace_id = Some(trace_id_for_events.clone());
                                    run.metadata.org_id = Some(org_id_for_events.clone());
                                    run.metadata.host_id = Some(host_id_clone.clone());
                                    run.mark_started();
                                    run.mark_failed(
                                        event
                                            .event
                                            .content
                                            .clone()
                                            .unwrap_or_else(|| "Unknown error".to_string()),
                                    );
                                    run.event_count = event_count;

                                    let session_end_event = execution_session_end_event(
                                        run_id,
                                        task_id,
                                        false,
                                        event.event.timestamp,
                                        run.started_at.unwrap_or(run.created_at),
                                    );
                                    if let Err(err) = state_clone
                                        .executor()
                                        .run_store()
                                        .append_event(task_id, run_id, &session_end_event)
                                    {
                                        tracing::warn!(
                                            "Failed to persist failed session event for task {} run {}: {}",
                                            task_id_str,
                                            run_id,
                                            err
                                        );
                                    } else {
                                        event_count = event_count.saturating_add(1);
                                        run.event_count = event_count;
                                    }

                                    if let Err(e) =
                                        state_clone.executor().run_store().save_run(&run)
                                    {
                                        tracing::warn!(
                                            "Failed to save failed run for task {}: {}",
                                            task_id_str,
                                            e
                                        );
                                    } else {
                                        tracing::info!(
                                            "Run {} failed for gateway task {}",
                                            run_id,
                                            task_id_str
                                        );
                                    }

                                    // Send error message
                                    {
                                        #[derive(serde::Serialize)]
                                        struct MessagePayload {
                                            #[serde(rename = "taskId")]
                                            task_id: String,
                                            message: TaskMessage,
                                        }
                                        #[derive(serde::Serialize)]
                                        struct TaskMessage {
                                            id: String,
                                            role: String,
                                            content: String,
                                            timestamp: i64,
                                            #[serde(rename = "isStreaming")]
                                            is_streaming: bool,
                                        }

                                        let error_content = event
                                            .event
                                            .content
                                            .clone()
                                            .unwrap_or_else(|| "浠诲姟鎵ц澶辫触".to_string());

                                        let error_msg_content = format!("鉂?{}", error_content);
                                        let msg_timestamp = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_millis()
                                            as i64;

                                        let _ = io.emit(
                                            "task:message",
                                            &MessagePayload {
                                                task_id: task_id_str.clone(),
                                                message: TaskMessage {
                                                    id: message_id.clone(),
                                                    role: "system".to_string(),
                                                    content: error_msg_content.clone(),
                                                    timestamp: msg_timestamp,
                                                    is_streaming: false,
                                                },
                                            },
                                        );

                                        // Persist the error message to disk
                                        let chat_msg = ChatMessage::with_id(
                                            message_id.clone(),
                                            MessageRole::System,
                                            error_msg_content,
                                        );
                                        if let Err(e) = state_clone
                                            .executor()
                                            .run_store()
                                            .append_message(task_id, run_id, &chat_msg)
                                        {
                                            tracing::warn!(
                                                "Failed to persist error message for task {}: {}",
                                                task_id_str,
                                                e
                                            );
                                        }
                                    }

                                    // Broadcast kanban sync
                                    let kanban_store = state_clone.kanban_store();
                                    let board_state = kanban_store.get_state().await;
                                    let _ = io.emit("kanban:sync", &board_state);
                                    tracing::info!(
                                        "Broadcasted kanban:sync after task {} failed",
                                        task_id_str
                                    );
                                    break;
                                }
                                _ => {}
                            }

                            // Also emit as execution_event for the ExecutionLogPanel
                            #[derive(serde::Serialize)]
                            struct ExecutionEventBase {
                                #[serde(rename = "executionId")]
                                execution_id: String,
                                #[serde(rename = "orgId")]
                                org_id: String,
                                #[serde(rename = "traceId")]
                                trace_id: String,
                                seq: u32,
                                ts: u64,
                                task_id: String,
                                event_type: String,
                                content: Option<String>,
                                timestamp: u64,
                            }
                            let _ = io.emit(
                                "task:execution_event",
                                &ExecutionEventBase {
                                    execution_id: run_id.to_string(),
                                    org_id: org_id_for_events.clone(),
                                    trace_id: trace_id_for_events.clone(),
                                    seq: event_count,
                                    ts: event.event.timestamp,
                                    task_id: task_id_str.clone(),
                                    event_type: format!("{:?}", event.event.event_type)
                                        .to_lowercase(),
                                    content: event.event.content.clone(),
                                    timestamp: event.event.timestamp,
                                },
                            );
                        }
                    }
                }
            });

            Ok(StartTaskExecutionResult {
                execution_id: run_id,
                task_id,
                status: "dispatched".to_string(),
                message: format!("Task dispatched to gateway host: {}", host_id),
                host_id,
                trace_id: trace_id_for_response,
                org_id: org_id_for_response,
            })
        }
        Err(e) => {
            tracing::error!("Failed to dispatch task to gateway: {}", e);
            let status = if e.starts_with("Host") {
                StatusCode::CONFLICT
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            };
            Err((
                status,
                Json(ErrorResponse {
                    error: format!("Gateway dispatch failed: {}", e),
                }),
            ))
        }
    }
}

/// GET /api/tasks/:id/status - Get execution status
async fn get_execution_status(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<SessionResponse>, RouteError> {
    if let Some(session) = state.executor().get_session_by_task(task_id).await {
        let session = session.read().await;
        let session_state = session.state().await;
        let status = session.status().await;

        return Ok(Json(SessionResponse {
            session_id: session.id,
            task_id: session.task_id,
            status: format!("{:?}", status).to_lowercase(),
            state: state_to_string(&session_state),
            worktree_path: session
                .worktree_path()
                .map(|p| p.to_string_lossy().to_string()),
            branch: session.worktree.as_ref().map(|w| w.branch.clone()),
        }));
    }

    let runs = state
        .executor()
        .list_runs(task_id)
        .map_err(internal_error)?;
    let Some(latest_run) = runs.first() else {
        return Err(not_found(format!(
            "No execution found for task {}",
            task_id
        )));
    };

    Ok(Json(SessionResponse {
        session_id: latest_run.id,
        task_id: latest_run.task_id,
        status: format!("{:?}", latest_run.status).to_lowercase(),
        state: run_status_to_state(latest_run.status),
        worktree_path: None,
        branch: None,
    }))
}

/// POST /api/tasks/:id/stop - Stop task execution
async fn stop_execution(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<Uuid>,
) -> Result<Json<ExecutionResponse>, RouteError> {
    let identity = resolve_user_identity(&headers, feature_multi_tenant()).map_err(unauthorized)?;
    if let Some(session) = state.executor().get_session_by_task(task_id).await {
        let runs = state
            .executor()
            .list_runs(task_id)
            .map_err(internal_error)?;
        if let Some(latest_run) = runs.first() {
            let persisted_run = state
                .executor()
                .run_store()
                .load_run(task_id, latest_run.id)
                .map_err(internal_error)?;
            ensure_org_scope(persisted_run.metadata.org_id.clone(), identity.as_ref())?;
        } else if identity.is_some() {
            return Err(not_found(format!(
                "No execution found for task {}",
                task_id
            )));
        }

        let session_id = session.read().await.id;
        state
            .executor()
            .cancel_session(session_id)
            .await
            .map_err(internal_error)?;
        record_audit_event(
            &state,
            AuditEvent::new(
                default_org_id(),
                "api",
                "task.stop.legacy",
                Some(session_id),
                Some(task_id),
                None,
                None,
                Some("cancelled".to_string()),
                json!({
                    "source": "/api/tasks/{id}/stop",
                    "deprecated": true,
                    "path": "active_session"
                }),
            ),
        )
        .await;
        return Ok(Json(ExecutionResponse {
            session_id,
            task_id,
            status: "cancelled".to_string(),
            message: "Execution cancelled".to_string(),
        }));
    }

    let runs = state
        .executor()
        .list_runs(task_id)
        .map_err(internal_error)?;
    let Some(latest_run) = runs.first() else {
        return Err(not_found(format!(
            "No execution found for task {}",
            task_id
        )));
    };

    let mut persisted_run = state
        .executor()
        .run_store()
        .load_run(task_id, latest_run.id)
        .map_err(internal_error)?;
    ensure_org_scope(persisted_run.metadata.org_id.clone(), identity.as_ref())?;

    let abort_result = state
        .gateway_manager()
        .abort_task(&task_id.to_string())
        .await;
    match abort_result {
        Ok(_) => {
            if !persisted_run.is_terminal() {
                persisted_run.mark_cancelled();
                if let Err(err) = state.executor().run_store().save_run(&persisted_run) {
                    tracing::warn!(
                        "Failed to persist cancelled run {} for task {}: {}",
                        persisted_run.id,
                        task_id,
                        err
                    );
                }
            }
            record_audit_event(
                &state,
                AuditEvent::new(
                    persisted_run
                        .metadata
                        .org_id
                        .clone()
                        .unwrap_or_else(default_org_id),
                    "api",
                    "task.stop.legacy",
                    Some(persisted_run.id),
                    Some(task_id),
                    persisted_run.metadata.host_id.clone(),
                    persisted_run.metadata.trace_id.clone(),
                    Some("cancelled".to_string()),
                    json!({
                        "source": "/api/tasks/{id}/stop",
                        "deprecated": true,
                        "path": "gateway_abort_ok"
                    }),
                ),
            )
            .await;
            Ok(Json(ExecutionResponse {
                session_id: persisted_run.id,
                task_id,
                status: "cancelled".to_string(),
                message: "Execution cancelled".to_string(),
            }))
        }
        Err(err) if err.contains("not found on any host") => {
            if !persisted_run.is_terminal() {
                persisted_run.mark_cancelled();
                if let Err(save_err) = state.executor().run_store().save_run(&persisted_run) {
                    tracing::warn!(
                        "Failed to persist inferred cancellation for run {} task {}: {}",
                        persisted_run.id,
                        task_id,
                        save_err
                    );
                }
            }
            record_audit_event(
                &state,
                AuditEvent::new(
                    persisted_run
                        .metadata
                        .org_id
                        .clone()
                        .unwrap_or_else(default_org_id),
                    "api",
                    "task.stop.legacy",
                    Some(persisted_run.id),
                    Some(task_id),
                    persisted_run.metadata.host_id.clone(),
                    persisted_run.metadata.trace_id.clone(),
                    Some("cancelled".to_string()),
                    json!({
                        "source": "/api/tasks/{id}/stop",
                        "deprecated": true,
                        "path": "gateway_abort_not_found"
                    }),
                ),
            )
            .await;
            Ok(Json(ExecutionResponse {
                session_id: persisted_run.id,
                task_id,
                status: "cancelled".to_string(),
                message: "Execution already stopped".to_string(),
            }))
        }
        Err(err) => Err(conflict_error(format!("Failed to stop execution: {}", err))),
    }
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

/// POST /api/tasks/:id/input - Send input to task
async fn send_input(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<Uuid>,
    Json(req): Json<SendInputRequest>,
) -> Result<StatusCode, RouteError> {
    let identity = resolve_user_identity(&headers, feature_multi_tenant()).map_err(unauthorized)?;
    let runs = state
        .executor()
        .list_runs(task_id)
        .map_err(internal_error)?;
    if let Some(latest_run) = runs.first() {
        let persisted_run = state
            .executor()
            .run_store()
            .load_run(task_id, latest_run.id)
            .map_err(internal_error)?;
        ensure_org_scope(persisted_run.metadata.org_id.clone(), identity.as_ref())?;
    } else if identity.is_some() {
        return Err(not_found(format!(
            "No execution found for task {}",
            task_id
        )));
    }

    let content = req.content;
    match state.executor().send_input(task_id, content.clone()).await {
        Ok(()) => {
            record_audit_event(
                &state,
                AuditEvent::new(
                    default_org_id(),
                    "api",
                    "task.input.legacy",
                    None,
                    Some(task_id),
                    None,
                    None,
                    Some("forwarded_local".to_string()),
                    json!({
                        "source": "/api/tasks/{id}/input",
                        "deprecated": true,
                        "contentLength": content.len()
                    }),
                ),
            )
            .await;
            Ok(StatusCode::OK)
        }
        Err(local_err) => {
            let gateway_result = state
                .gateway_manager()
                .send_input(&task_id.to_string(), content)
                .await;
            match gateway_result {
                Ok(()) => {
                    record_audit_event(
                        &state,
                        AuditEvent::new(
                            default_org_id(),
                            "api",
                            "task.input.legacy",
                            None,
                            Some(task_id),
                            None,
                            None,
                            Some("forwarded_gateway".to_string()),
                            json!({
                                "source": "/api/tasks/{id}/input",
                                "deprecated": true
                            }),
                        ),
                    )
                    .await;
                    Ok(StatusCode::OK)
                }
                Err(gateway_err) => Err(conflict_error(format!(
                    "Failed to send input (local: {}; gateway: {})",
                    local_err, gateway_err
                ))),
            }
        }
    }
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

    Ok(Json(SessionListResponse {
        sessions: summaries,
    }))
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
        worktree_path: session
            .worktree_path()
            .map(|p| p.to_string_lossy().to_string()),
        branch: session.worktree.as_ref().map(|w| w.branch.clone()),
    }))
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

fn validation_error(error: impl Into<String>) -> RouteError {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
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

fn normalize_org_id(org_id: &str) -> String {
    let normalized = org_id.trim();
    if normalized.is_empty() {
        default_org_id()
    } else {
        normalized.to_string()
    }
}

fn ensure_org_scope(
    org_id: Option<String>,
    identity: Option<&UserIdentity>,
) -> Result<(), RouteError> {
    let Some(identity) = identity else {
        return Ok(());
    };
    let run_org_id = org_id.unwrap_or_else(default_org_id);
    if run_org_id != identity.org_id {
        return Err(not_found("Execution not found"));
    }
    Ok(())
}

async fn record_audit_event(state: &AppState, event: AuditEvent) {
    if let Err(err) = state.audit_store().append(event).await {
        tracing::warn!("Failed to append audit event: {}", err);
    }
}

fn run_status_to_state(status: ExecutionStatus) -> String {
    match status {
        ExecutionStatus::Initializing
        | ExecutionStatus::CreatingWorktree
        | ExecutionStatus::Starting
        | ExecutionStatus::Running => "running".to_string(),
        ExecutionStatus::Paused => "paused".to_string(),
        ExecutionStatus::Completed => "completed(0)".to_string(),
        ExecutionStatus::Failed => "failed".to_string(),
        ExecutionStatus::Cancelled => "cancelled".to_string(),
        ExecutionStatus::CleaningUp => "cleaning_up".to_string(),
    }
}

fn timestamp_from_millis(ts: u64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp_millis(ts as i64).unwrap_or_else(Utc::now)
}

fn execution_event_from_gateway(
    run_id: Uuid,
    task_id: Uuid,
    event: &GatewayAgentEvent,
) -> ExecutionEvent {
    let content = event.content.clone().unwrap_or_default();
    let payload = match event.event_type {
        GatewayAgentEventType::Thinking => agent_runner::AgentEvent::Thinking { content },
        GatewayAgentEventType::Message => agent_runner::AgentEvent::Message { content },
        GatewayAgentEventType::Error | GatewayAgentEventType::Failed => {
            agent_runner::AgentEvent::Error {
                message: if content.is_empty() {
                    "Gateway execution error".to_string()
                } else {
                    content
                },
                recoverable: false,
            }
        }
        GatewayAgentEventType::Completed => agent_runner::AgentEvent::Completed {
            success: true,
            summary: event.content.clone(),
        },
        GatewayAgentEventType::ToolCall => agent_runner::AgentEvent::Message {
            content: format!("tool_call {}", event.data),
        },
        GatewayAgentEventType::ToolResult => agent_runner::AgentEvent::Message {
            content: format!("tool_result {}", event.data),
        },
        GatewayAgentEventType::FileChange => agent_runner::AgentEvent::Message {
            content: format!("file_change {}", event.data),
        },
        GatewayAgentEventType::Log
        | GatewayAgentEventType::Stdout
        | GatewayAgentEventType::Stderr
        | GatewayAgentEventType::Output => agent_runner::AgentEvent::Message {
            content: if content.is_empty() {
                event.data.to_string()
            } else {
                content
            },
        },
    };

    let mut execution_event = ExecutionEvent::agent_event(run_id, task_id, payload);
    execution_event.timestamp = timestamp_from_millis(event.timestamp);
    execution_event
}

fn execution_session_end_event(
    run_id: Uuid,
    task_id: Uuid,
    success: bool,
    event_ts: u64,
    started_at: DateTime<Utc>,
) -> ExecutionEvent {
    let ended_at = timestamp_from_millis(event_ts);
    let duration_ms = ended_at
        .signed_duration_since(started_at)
        .num_milliseconds()
        .max(0) as u64;
    let status = if success {
        ExecutionStatus::Completed
    } else {
        ExecutionStatus::Failed
    };

    let mut end_event = ExecutionEvent::session_ended(run_id, task_id, status, duration_ms);
    end_event.timestamp = ended_at;
    end_event
}

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
        .route("/api/tasks/{id}/input", post(send_input))
        .route("/api/tasks/{id}/worktree", delete(cleanup_worktree))
        // Session endpoints
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions/{id}", get(get_session))
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
    use std::{collections::HashMap, sync::Arc};

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
        task::{FileTaskStore, Task},
        workspace::CreateWorkspaceRequest,
    };

    use crate::{
        gateway::{
            protocol::{HostCapabilities, ServerToGatewayMessage},
            GatewayManager,
        },
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

    fn execution_body() -> Body {
        Body::from(
            json!({
                "agentType": "opencode",
                "baseBranch": "main"
            })
            .to_string(),
        )
    }

    #[test]
    fn start_execution_request_deserializes_camel_case() {
        let value = serde_json::json!({
            "agentType": "opencode",
            "baseBranch": "main"
        });
        let req: StartExecutionRequest = serde_json::from_value(value).expect("valid payload");
        assert_eq!(req.agent_type, "opencode");
        assert_eq!(req.base_branch, "main");
    }

    #[tokio::test]
    async fn start_execution_without_project_id_returns_unprocessable_entity() {
        let (state, _temp_dir) = build_state().await;
        let task = state
            .task_store()
            .create(Task::new("No project".to_string()))
            .await
            .unwrap();

        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/tasks/{}/execute", task.id))
                    .header("Content-Type", "application/json")
                    .body(execution_body())
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
    async fn start_execution_with_missing_project_returns_not_found() {
        let (state, _temp_dir) = build_state().await;
        let task = state
            .task_store()
            .create(Task::new("Unknown project".to_string()).with_project_id(Uuid::new_v4()))
            .await
            .unwrap();

        let app = router().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/tasks/{}/execute", task.id))
                    .header("Content-Type", "application/json")
                    .body(execution_body())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn start_execution_rejects_mismatched_workspace_binding_with_conflict() {
        let (state, _temp_dir) = build_state().await;
        let project_workspace_id = state.workspace_store().list().await[0].id;
        let mismatched_workspace = state
            .workspace_store()
            .create(CreateWorkspaceRequest {
                name: "other-workspace".to_string(),
                slug: Some("other-workspace".to_string()),
                host_id: "host-other".to_string(),
                root_path: "/tmp/other-workspace".to_string(),
                default_project_id: None,
                org_id: None,
            })
            .await
            .unwrap();
        let project = state
            .project_store()
            .register(
                "host-bound".to_string(),
                CreateProjectRequest {
                    name: "workspace-bound-project".to_string(),
                    local_path: "/tmp/workspace-bound-project".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id: project_workspace_id,
                    org_id: None,
                },
            )
            .await
            .unwrap();
        let task = state
            .task_store()
            .create(
                Task::new("Mismatched binding".to_string())
                    .with_project_binding(project.id, mismatched_workspace.id),
            )
            .await
            .unwrap();

        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        state
            .gateway_manager()
            .register_host(
                project.gateway_id.clone(),
                HostCapabilities {
                    name: "Bound host".to_string(),
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
                    .uri(format!("/api/tasks/{}/execute", task.id))
                    .header("Content-Type", "application/json")
                    .body(execution_body())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn start_execution_accepts_matching_workspace_binding() {
        let (state, _temp_dir) = build_state().await;
        let workspace_id = state.workspace_store().list().await[0].id;
        let project = state
            .project_store()
            .register(
                "host-bound".to_string(),
                CreateProjectRequest {
                    name: "matching-workspace-project".to_string(),
                    local_path: "/tmp/matching-workspace-project".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id,
                    org_id: None,
                },
            )
            .await
            .unwrap();
        let task = state
            .task_store()
            .create(
                Task::new("Matching binding".to_string())
                    .with_project_binding(project.id, project.workspace_id),
            )
            .await
            .unwrap();

        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        state
            .gateway_manager()
            .register_host(
                project.gateway_id.clone(),
                HostCapabilities {
                    name: "Bound host".to_string(),
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
                    .uri(format!("/api/tasks/{}/execute", task.id))
                    .header("Content-Type", "application/json")
                    .body(execution_body())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn start_execution_backfills_missing_workspace_binding_before_dispatch() {
        let (state, _temp_dir) = build_state().await;
        let workspace_id = state.workspace_store().list().await[0].id;
        let project = state
            .project_store()
            .register(
                "host-backfill".to_string(),
                CreateProjectRequest {
                    name: "backfill-workspace-project".to_string(),
                    local_path: "/tmp/backfill-workspace-project".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id,
                    org_id: None,
                },
            )
            .await
            .unwrap();
        let task = state
            .task_store()
            .create(Task::new("Backfill binding".to_string()).with_project_id(project.id))
            .await
            .unwrap();

        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        state
            .gateway_manager()
            .register_host(
                project.gateway_id.clone(),
                HostCapabilities {
                    name: "Bound host".to_string(),
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
                    .uri(format!("/api/tasks/{}/execute", task.id))
                    .header("Content-Type", "application/json")
                    .body(execution_body())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);

        let updated_task = state.task_store().get(task.id).await.unwrap().unwrap();
        assert_eq!(updated_task.workspace_id, Some(project.workspace_id));
    }

    #[tokio::test]
    async fn start_execution_with_offline_project_host_returns_conflict() {
        let (state, _temp_dir) = build_state().await;
        let workspace_id = state.workspace_store().list().await[0].id;
        let project = state
            .project_store()
            .register(
                "host-offline".to_string(),
                CreateProjectRequest {
                    name: "offline-project".to_string(),
                    local_path: "/tmp/offline-project".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id,
                    org_id: None,
                },
            )
            .await
            .unwrap();
        let task = state
            .task_store()
            .create(Task::new("Offline host".to_string()).with_project_id(project.id))
            .await
            .unwrap();

        let app = router().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/tasks/{}/execute", task.id))
                    .header("Content-Type", "application/json")
                    .body(execution_body())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn start_execution_dispatches_project_cwd_to_bound_host() {
        let (state, _temp_dir) = build_state().await;
        let workspace_id = state.workspace_store().list().await[0].id;
        let project = state
            .project_store()
            .register(
                "host-dispatch".to_string(),
                CreateProjectRequest {
                    name: "bound-project".to_string(),
                    local_path: "/tmp/bound-project".to_string(),
                    remote_url: None,
                    default_branch: Some("develop".to_string()),
                    worktree_dir: None,
                    workspace_id,
                    org_id: None,
                },
            )
            .await
            .unwrap();
        let task = state
            .task_store()
            .create(Task::new("Dispatch cwd".to_string()).with_project_id(project.id))
            .await
            .unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::channel(10);
        state
            .gateway_manager()
            .register_host(
                project.gateway_id.clone(),
                HostCapabilities {
                    name: "Bound host".to_string(),
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
                    .uri(format!("/api/tasks/{}/execute", task.id))
                    .header("Content-Type", "application/json")
                    .body(execution_body())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let run_id = Uuid::parse_str(payload["sessionId"].as_str().unwrap()).unwrap();

        let msg = rx.recv().await.unwrap();
        match msg {
            ServerToGatewayMessage::TaskExecute { task } => {
                assert_eq!(task.cwd, project.local_path);
                assert_eq!(task.metadata["projectId"], project.id.to_string());
                assert_eq!(
                    task.metadata["workspaceId"],
                    project.workspace_id.to_string()
                );
            }
            _ => panic!("expected task dispatch message"),
        }

        let run = state
            .executor()
            .run_store()
            .load_run(task.id, run_id)
            .unwrap();
        assert_eq!(run.metadata.project_id, Some(project.id));
        assert_eq!(run.metadata.workspace_id, Some(project.workspace_id));
    }
}
