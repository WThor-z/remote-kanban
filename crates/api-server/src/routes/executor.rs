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
use std::collections::HashMap;
use uuid::Uuid;

use agent_runner::{AgentType, ChatMessage, MessageRole, Run, SessionState};
use vk_core::kanban::KanbanTaskStatus;
use vk_core::task::TaskRepository;

use crate::gateway::protocol::GatewayTaskRequest;
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
    // Log received request for debugging
    tracing::info!(
        "Execute request for task {}: agent_type={}, target_host={:?}, model={:?}",
        task_id, req.agent_type, req.target_host, req.model
    );

    // Verify task exists
    let mut task = state
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

    let project_id = task.project_id.ok_or_else(|| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: "Project is required".to_string(),
            }),
        )
    })?;

    let project = state.project_store().get(project_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Project {} not found", project_id),
            }),
        )
    })?;

    match task.workspace_id {
        Some(workspace_id) if workspace_id != project.workspace_id => {
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: format!(
                        "Task workspace {} does not match project workspace {}",
                        workspace_id, project.workspace_id
                    ),
                }),
            ));
        }
        Some(_) => {}
        None => {
            task.workspace_id = Some(project.workspace_id);
            task = state.task_store().update(task).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: e.to_string(),
                    }),
                )
            })?;
        }
    }

    // Build prompt from task
    let prompt = if let Some(desc) = &task.description {
        format!("{}\n\n{}", task.title, desc)
    } else {
        task.title.clone()
    };

    let base_branch = task
        .base_branch
        .clone()
        .unwrap_or_else(|| project.default_branch.clone());

    if let Some(target_host) = &req.target_host {
        tracing::info!(
            "Ignoring requested target_host={} for task {} because execution is project-bound",
            target_host,
            task_id
        );
    }

    dispatch_to_gateway(
        &state,
        task_id,
        &prompt,
        &req.agent_type,
        &project.gateway_id,
        &task.title,
        task.description.as_deref(),
        &project.local_path,
        req.model.as_deref(),
        &base_branch,
        project.id,
        project.workspace_id,
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
) -> Result<(StatusCode, Json<ExecutionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let gateway_manager = state.gateway_manager();
    let memory_settings_snapshot = serde_json::to_value(state.memory_store().get_settings().await).ok();

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
            let parsed_agent_type = AgentType::from_str(agent_type)
                .unwrap_or(AgentType::OpenCode);
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
            run.mark_started();
            
            // Save the initial run record
            if let Err(e) = state.executor().run_store().save_run(&run) {
                tracing::warn!("Failed to save initial run record for gateway task {}: {}", task_id, e);
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
            tokio::spawn(async move {
                let mut event_rx = state_clone.gateway_manager().subscribe();
                let io = state_clone.get_socket_io().await;
                let mut event_count: u32 = 0;
                // Accumulate stdout content for final message
                let mut accumulated_output = String::new();
                // Fixed message ID for streaming updates
                let message_id = uuid::Uuid::new_v4().to_string();
                
                if let Some(io) = io {
                    // Move task to Doing when execution starts
                    {
                        let kanban_store = state_clone.kanban_store();
                        if let Err(e) = kanban_store.move_task(&task_id_str, KanbanTaskStatus::Doing, None).await {
                            tracing::warn!("Failed to move kanban task {} to Doing: {}", task_id_str, e);
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
                        
                        let _ = io.emit("task:message", &MessagePayload {
                            task_id: task_id_str.clone(),
                            message: TaskMessage {
                                id: message_id.clone(),
                                role: "assistant".to_string(),
                                content: "正在工作中...".to_string(),
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as i64,
                                is_streaming: true,
                            },
                        });
                    }
                    
                    while let Ok(event) = event_rx.recv().await {
                        if event.task_id == task_id_str {
                            event_count += 1;
                            
                            // Forward gateway event to Socket.IO for Logs panel
                            let _ = io.emit("task:gateway_event", &event);
                            
                            // Accumulate stdout content
                            if let Some(content) = &event.event.content {
                                match event.event.event_type {
                                    crate::gateway::protocol::GatewayAgentEventType::Stdout => {
                                        if !content.starts_with("[executor]") && 
                                           !content.starts_with("[Gateway]") &&
                                           !content.is_empty() {
                                            accumulated_output.push_str(content);
                                        }
                                    }
                                    crate::gateway::protocol::GatewayAgentEventType::Message => {
                                        accumulated_output.push_str(content);
                                    }
                                    _ => {}
                                }
                            }
                            
                            // Check for Completed/Failed events and update Run record
                            match event.event.event_type {
                                crate::gateway::protocol::GatewayAgentEventType::Completed => {
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
                                    run.mark_started();
                                    run.mark_completed(0, event.event.content.clone());
                                    run.event_count = event_count;
                                    
                                    if let Err(e) = state_clone.executor().run_store().save_run(&run) {
                                        tracing::warn!("Failed to save completed run for task {}: {}", task_id_str, e);
                                    } else {
                                        tracing::info!("Run {} completed for gateway task {}", run_id, task_id_str);
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
                                            event.event.content.clone().unwrap_or_else(|| "任务完成".to_string())
                                        } else {
                                            accumulated_output.clone()
                                        };
                                        
                                        let msg_timestamp = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_millis() as i64;
                                        
                                        let _ = io.emit("task:message", &MessagePayload {
                                            task_id: task_id_str.clone(),
                                            message: TaskMessage {
                                                id: message_id.clone(),
                                                role: "assistant".to_string(),
                                                content: final_content.clone(),
                                                timestamp: msg_timestamp,
                                                is_streaming: false,
                                            },
                                        });
                                        
                                        // Persist the message to disk
                                        let chat_msg = ChatMessage::with_id(
                                            message_id.clone(),
                                            MessageRole::Assistant,
                                            final_content,
                                        );
                                        if let Err(e) = state_clone.executor().run_store().append_message(task_id, run_id, &chat_msg) {
                                            tracing::warn!("Failed to persist message for task {}: {}", task_id_str, e);
                                        }
                                    }
                                    
                                    // Broadcast kanban sync
                                    let kanban_store = state_clone.kanban_store();
                                    let board_state = kanban_store.get_state().await;
                                    let _ = io.emit("kanban:sync", &board_state);
                                    tracing::info!("Broadcasted kanban:sync after task {} completed", task_id_str);
                                    break;
                                }
                                crate::gateway::protocol::GatewayAgentEventType::Failed => {
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
                                    run.mark_started();
                                    run.mark_failed(event.event.content.clone().unwrap_or_else(|| "Unknown error".to_string()));
                                    run.event_count = event_count;
                                    
                                    if let Err(e) = state_clone.executor().run_store().save_run(&run) {
                                        tracing::warn!("Failed to save failed run for task {}: {}", task_id_str, e);
                                    } else {
                                        tracing::info!("Run {} failed for gateway task {}", run_id, task_id_str);
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
                                        
                                        let error_content = event.event.content.clone()
                                            .unwrap_or_else(|| "任务执行失败".to_string());
                                        
                                        let error_msg_content = format!("❌ {}", error_content);
                                        let msg_timestamp = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_millis() as i64;
                                        
                                        let _ = io.emit("task:message", &MessagePayload {
                                            task_id: task_id_str.clone(),
                                            message: TaskMessage {
                                                id: message_id.clone(),
                                                role: "system".to_string(),
                                                content: error_msg_content.clone(),
                                                timestamp: msg_timestamp,
                                                is_streaming: false,
                                            },
                                        });
                                        
                                        // Persist the error message to disk
                                        let chat_msg = ChatMessage::with_id(
                                            message_id.clone(),
                                            MessageRole::System,
                                            error_msg_content,
                                        );
                                        if let Err(e) = state_clone.executor().run_store().append_message(task_id, run_id, &chat_msg) {
                                            tracing::warn!("Failed to persist error message for task {}: {}", task_id_str, e);
                                        }
                                    }
                                    
                                    // Broadcast kanban sync
                                    let kanban_store = state_clone.kanban_store();
                                    let board_state = kanban_store.get_state().await;
                                    let _ = io.emit("kanban:sync", &board_state);
                                    tracing::info!("Broadcasted kanban:sync after task {} failed", task_id_str);
                                    break;
                                }
                                _ => {}
                            }
                            
                            // Also emit as execution_event for the ExecutionLogPanel
                            #[derive(serde::Serialize)]
                            struct ExecutionEventBase {
                                task_id: String,
                                event_type: String,
                                content: Option<String>,
                                timestamp: u64,
                            }
                            let _ = io.emit("task:execution_event", &ExecutionEventBase {
                                task_id: task_id_str.clone(),
                                event_type: format!("{:?}", event.event.event_type).to_lowercase(),
                                content: event.event.content.clone(),
                                timestamp: event.event.timestamp,
                            });
                        }
                    }
                }
            });

            Ok((
                StatusCode::ACCEPTED,
                Json(ExecutionResponse {
                    session_id: run_id, // Use the run_id as session_id for consistency
                    task_id,
                    status: "dispatched".to_string(),
                    message: format!("Task dispatched to gateway host: {}", host_id),
                }),
            ))
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

/// POST /api/tasks/:id/input - Send input to task
async fn send_input(
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    Json(req): Json<SendInputRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state
        .executor()
        .send_input(task_id, req.content)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(StatusCode::OK)
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
        .route("/api/tasks/{id}/input", post(send_input))
        .route("/api/tasks/{id}/worktree", delete(cleanup_worktree))
        // Session endpoints
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions/{id}", get(get_session))
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
        workspace::CreateWorkspaceRequest,
        task::{FileTaskStore, Task},
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
                assert_eq!(task.metadata["workspaceId"], project.workspace_id.to_string());
            }
            _ => panic!("expected task dispatch message"),
        }

        let run = state.executor().run_store().load_run(task.id, run_id).unwrap();
        assert_eq!(run.metadata.project_id, Some(project.id));
        assert_eq!(run.metadata.workspace_id, Some(project.workspace_id));
    }
}
