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

use agent_runner::{AgentType, ChatMessage, ExecuteRequest, MessageRole, Run, SessionState};
use vk_core::kanban::KanbanTaskStatus;
use vk_core::task::{TaskRepository, TaskStatus};

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

    // Check if we should dispatch to a remote Gateway
    if let Some(target_host) = &req.target_host {
        return dispatch_to_gateway(&state, task_id, &prompt, &req.agent_type, target_host, req.model.as_deref()).await;
    }

    // Otherwise, try to auto-select a gateway or fall back to local execution
    let gateway_manager = state.gateway_manager();
    let hosts = gateway_manager.list_hosts().await;
    
    // Check if there's an available gateway host for this agent type
    let available_host = hosts.iter().find(|h| {
        h.capabilities.agents.contains(&req.agent_type) 
            && h.status == crate::gateway::protocol::HostConnectionStatus::Online
    });

    if let Some(host) = available_host {
        tracing::info!("Auto-selecting gateway host {} for task {}", host.host_id, task_id);
        return dispatch_to_gateway(&state, task_id, &prompt, &req.agent_type, &host.host_id, req.model.as_deref()).await;
    }

    // Fall back to local execution
    execute_locally(state, task_id, req.agent_type, req.base_branch, prompt).await
}

/// Dispatch task to a remote Gateway host
async fn dispatch_to_gateway(
    state: &AppState,
    task_id: Uuid,
    prompt: &str,
    agent_type: &str,
    _target_host: &str,
    model: Option<&str>,
) -> Result<(StatusCode, Json<ExecutionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let gateway_manager = state.gateway_manager();
    
    // 不发送 CWD，让 Gateway 使用自己配置的工作目录
    // 这样可以避免与本地 OpenCode 实例的锁文件冲突
    let gateway_task = GatewayTaskRequest {
        task_id: task_id.to_string(),
        prompt: prompt.to_string(),
        cwd: String::new(),  // 空字符串，让 Gateway 使用自己的 defaultCwd
        agent_type: agent_type.to_string(),
        model: model.map(String::from),
        env: HashMap::new(),
        timeout: None,
        metadata: serde_json::Value::Null,
    };

    match gateway_manager.dispatch_task(gateway_task).await {
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
                "main".to_string(), // Gateway doesn't use worktrees, use default branch
            );
            // Override the generated ID to use our run_id
            run.id = run_id;
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
                                        "main".to_string(),
                                    );
                                    run.id = run_id;
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
                                        "main".to_string(),
                                    );
                                    run.id = run_id;
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
            Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: format!("Gateway dispatch failed: {}", e),
                }),
            ))
        }
    }
}

/// Execute task locally using the built-in executor
async fn execute_locally(
    state: AppState,
    task_id: Uuid,
    agent_type: String,
    base_branch: String,
    prompt: String,
) -> Result<(StatusCode, Json<ExecutionResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Create execute request
    let execute_req = ExecuteRequest {
        task_id,
        agent_type,
        base_branch,
        prompt,
    };

    // Start execution
    let (session_id, mut event_rx) = state.executor().execute(execute_req).await.map_err(|e| {
        tracing::error!("Failed to start execution: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    // Spawn event bridge to Socket.IO
    let state_clone = state.clone();
    tokio::spawn(async move {
        let io = state_clone.get_socket_io().await;
        if let Some(io) = io {
            while let Some(event) = event_rx.recv().await {
                // Emit structured event
                // We emit to all clients. In production, we should emit to a room "task:{task_id}"
                let _ = io.emit("task:execution_event", &event);
                
                // Compatibility mapping for existing UI components
                match &event.event {
                    agent_runner::ExecutionEventType::StatusChanged { new_status, .. } => {
                        let status_str = match new_status {
                            agent_runner::ExecutionStatus::Initializing |
                            agent_runner::ExecutionStatus::CreatingWorktree |
                            agent_runner::ExecutionStatus::Starting => "starting",
                            agent_runner::ExecutionStatus::Running |
                            agent_runner::ExecutionStatus::CleaningUp => "running",
                            agent_runner::ExecutionStatus::Paused => "paused",
                            agent_runner::ExecutionStatus::Completed => "completed",
                            agent_runner::ExecutionStatus::Failed => "failed",
                            agent_runner::ExecutionStatus::Cancelled => "aborted",
                        };
                        
                        #[derive(Serialize)]
                        struct StatusPayload {
                            #[serde(rename = "taskId")]
                            task_id: Uuid,
                            status: String,
                        }
                        let _ = io.emit("task:status", &StatusPayload {
                            task_id: event.task_id,
                            status: status_str.to_string(),
                        });
                        
                        // Update task status in TaskStore based on execution status
                        let task_status = match new_status {
                            agent_runner::ExecutionStatus::Running => Some(TaskStatus::InProgress),
                            agent_runner::ExecutionStatus::Completed => Some(TaskStatus::Done),
                            agent_runner::ExecutionStatus::Failed |
                            agent_runner::ExecutionStatus::Cancelled => Some(TaskStatus::Done),
                            _ => None,
                        };
                        
                        if let Some(new_task_status) = task_status {
                            // Update TaskStore
                            let task_store = state_clone.task_store();
                            if let Ok(Some(mut task)) = task_store.get(event.task_id).await {
                                task.status = new_task_status;
                                if let Err(e) = task_store.update(task).await {
                                    tracing::warn!("Failed to update task status: {}", e);
                                } else {
                                    tracing::info!(
                                        "Task {} status updated to {:?}",
                                        event.task_id, new_task_status
                                    );
                                }
                            }
                            
                            // Update KanbanStore (this is what the UI actually reads)
                            let kanban_status = match new_task_status {
                                TaskStatus::InProgress => KanbanTaskStatus::Doing,
                                TaskStatus::Done => KanbanTaskStatus::Done,
                                TaskStatus::InReview => KanbanTaskStatus::Doing,
                                TaskStatus::Todo => KanbanTaskStatus::Todo,
                            };
                            
                            let kanban_store = state_clone.kanban_store();
                            let task_id_str = event.task_id.to_string();
                            if let Err(e) = kanban_store.move_task(&task_id_str, kanban_status, None).await {
                                tracing::warn!("Failed to move kanban task: {}", e);
                            } else {
                                tracing::info!(
                                    "Kanban task {} moved to {:?}",
                                    event.task_id, kanban_status
                                );
                                // Broadcast kanban sync to all clients
                                let board_state = kanban_store.get_state().await;
                                let _ = io.emit("kanban:sync", &board_state);
                            }
                        }
                    },
                    agent_runner::ExecutionEventType::AgentEvent { event: agent_event } => {
                        match agent_event {
                            agent_runner::AgentEvent::Message { content } | 
                            agent_runner::AgentEvent::Thinking { content } |
                            agent_runner::AgentEvent::Error { message: content, .. } |
                            agent_runner::AgentEvent::RawOutput { content, .. } => {
                                // Send as chat message
                                let role = match agent_event {
                                    agent_runner::AgentEvent::Thinking { .. } => "system",
                                    agent_runner::AgentEvent::Error { .. } => "system",
                                    _ => "assistant",
                                };

                                let content = match agent_event {
                                    agent_runner::AgentEvent::Thinking { .. } => format!("💭 {}", content),
                                    agent_runner::AgentEvent::Error { .. } => format!("❌ Error: {}", content),
                                    _ => content.clone(),
                                };
                                
                                emit_message(&io, event.task_id, event.id, role, content, event.timestamp.timestamp_millis());
                            },
                            agent_runner::AgentEvent::Command { command, output, .. } => {
                                let content = format!("$ {}\n{}", command, output);
                                emit_message(&io, event.task_id, event.id, "assistant", content, event.timestamp.timestamp_millis());
                            },
                            _ => {}
                        }
                    },
                    _ => {}
                }
            }
        }
    });

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

fn emit_message(io: &socketioxide::SocketIo, task_id: Uuid, id: Uuid, role: &str, content: String, timestamp: i64) {
    #[derive(Serialize)]
    struct MessagePayload {
        #[serde(rename = "taskId")]
        task_id: Uuid,
        message: TaskMessage,
    }
    #[derive(Serialize)]
    struct TaskMessage {
        id: String,
        role: String,
        content: String,
        timestamp: i64,
    }

    let _ = io.emit("task:message", &MessagePayload {
        task_id,
        message: TaskMessage {
            id: id.to_string(),
            role: role.to_string(),
            content,
            timestamp,
        }
    });
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
}
