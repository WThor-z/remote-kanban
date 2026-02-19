//! Socket.IO event handlers for real-time communication
//!
//! This module provides Socket.IO integration for the kanban board,
//! compatible with the existing frontend.

use serde::{Deserialize, Serialize};
use socketioxide::extract::{Data, SocketRef, State};
use socketioxide::{SocketIo, TransportType};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use vk_core::agent::{OpencodeClient, OpencodeConfig};
use vk_core::kanban::{KanbanStore, KanbanTaskStatus};
use vk_core::task::{FileTaskStore, TaskRepository};

/// Task execution session
#[allow(dead_code)]
struct TaskSession {
    task_id: String,
    client: OpencodeClient,
    status: String,
}

/// Shared state for Socket.IO handlers
#[derive(Clone)]
pub struct SocketState {
    pub kanban_store: Arc<KanbanStore>,
    pub task_store: Arc<FileTaskStore>,
    pub data_dir: PathBuf,
    /// Active task execution sessions
    sessions: Arc<RwLock<HashMap<String, Arc<RwLock<TaskSession>>>>>,
}

impl SocketState {
    pub fn new(
        kanban_store: Arc<KanbanStore>,
        task_store: Arc<FileTaskStore>,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            kanban_store,
            task_store,
            data_dir,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

// ============ Event Payloads ============

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskPayload {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveTaskPayload {
    pub task_id: String,
    pub target_status: KanbanTaskStatus,
    pub target_index: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTaskPayload {
    pub task_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskHistoryPayload {
    pub task_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskHistoryResponse {
    pub task_id: String,
    pub status: &'static str,
    pub messages: Vec<()>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload {
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStatusPayload {
    pub task_id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskErrorPayload {
    pub task_id: String,
    pub error: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskMessagePayload {
    pub task_id: String,
    pub message: TaskMessage,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStopPayload {
    pub task_id: String,
}

// ============ Event Handlers ============

/// Handle new socket connection
pub async fn on_connect(socket: SocketRef, State(_state): State<SocketState>) {
    let id = socket.id.to_string();
    info!("Client connected: {}", id);

    // Register event handlers
    socket.on(
        "kanban:request-sync",
        |socket: SocketRef, State(state): State<SocketState>| async move {
            handle_request_sync(socket, state).await;
        },
    );

    socket.on(
        "kanban:create",
        |socket: SocketRef,
         State(state): State<SocketState>,
         Data(data): Data<CreateTaskPayload>| async move {
            handle_create_task(socket, state, data).await;
        },
    );

    socket.on(
        "kanban:move",
        |socket: SocketRef, State(state): State<SocketState>, Data(data): Data<MoveTaskPayload>| async move {
            handle_move_task(socket, state, data).await;
        },
    );

    socket.on(
        "kanban:delete",
        |socket: SocketRef,
         State(state): State<SocketState>,
         Data(data): Data<DeleteTaskPayload>| async move {
            handle_delete_task(socket, state, data).await;
        },
    );

    socket.on_disconnect(|socket: SocketRef| async move {
        info!("Client disconnected: {}", socket.id);
    });

    // Task events
    socket.on(
        "task:history",
        |socket: SocketRef, Data(data): Data<TaskHistoryPayload>| async move {
            info!("Task history requested for: {}", data.task_id);
            // Return empty history - actual implementation in M-1.5+
            let response = TaskHistoryResponse {
                task_id: data.task_id,
                status: "idle",
                messages: vec![],
            };
            let _ = socket.emit("task:history", &response);
        },
    );

    socket.on(
        "task:execute",
        |socket: SocketRef,
         State(state): State<SocketState>,
         Data(data): Data<TaskHistoryPayload>| async move {
            let task_id = data.task_id.clone();
            info!("Task execution requested: {}", task_id);

            // Check if task is already running
            {
                let sessions = state.sessions.read().await;
                if sessions.contains_key(&task_id) {
                    warn!("Task {} is already running", task_id);
                    let _ = socket.emit(
                        "task:error",
                        &TaskErrorPayload {
                            task_id: task_id.clone(),
                            error: "Task is already running".to_string(),
                        },
                    );
                    return;
                }
            }

            // Send starting status
            let _ = socket.emit(
                "task:status",
                &TaskStatusPayload {
                    task_id: task_id.clone(),
                    status: "starting".to_string(),
                },
            );

            // Move task to "Doing" column immediately
            if let Err(e) = state
                .kanban_store
                .move_task(&task_id, KanbanTaskStatus::Doing, None)
                .await
            {
                warn!("Failed to move task to doing: {}", e);
            } else {
                // Broadcast kanban sync to all clients
                let board_state = state.kanban_store.get_state().await;
                let _ = socket.emit("kanban:sync", &board_state);
                let _ = socket.broadcast().emit("kanban:sync", &board_state);
            }

            // Get task info from kanban store
            let task_opt = state.kanban_store.get_task(&task_id).await;
            let (title, description) = match task_opt {
                Some(task) => (
                    task.title.clone(),
                    task.description.clone().unwrap_or_default(),
                ),
                None => {
                    warn!("Task not found: {}", task_id);
                    let _ = socket.emit(
                        "task:error",
                        &TaskErrorPayload {
                            task_id: task_id.clone(),
                            error: "Task not found".to_string(),
                        },
                    );
                    return;
                }
            };

            // Create prompt from task info
            let prompt = format!("{}\n\n{}", title, description);

            // Create OpenCode client
            let config = OpencodeConfig {
                cwd: state.data_dir.clone(),
                env: vec![],
            };
            let client = OpencodeClient::new(config);

            // Create task session
            let session = Arc::new(RwLock::new(TaskSession {
                task_id: task_id.clone(),
                client,
                status: "starting".to_string(),
            }));

            // Store session
            {
                let mut sessions = state.sessions.write().await;
                sessions.insert(task_id.clone(), session.clone());
            }

            // Clone what we need for the spawned task
            let socket_clone = socket.clone();
            let task_id_clone = task_id.clone();
            let sessions = state.sessions.clone();
            let kanban_store = state.kanban_store.clone();

            // Spawn task execution in background
            tokio::spawn(async move {
                // Get subscriber before starting
                let client_rx = {
                    let session_guard = session.read().await;
                    session_guard.client.subscribe()
                };

                // Update status to running
                let _ = socket_clone.emit(
                    "task:status",
                    &TaskStatusPayload {
                        task_id: task_id_clone.clone(),
                        status: "running".to_string(),
                    },
                );
                info!("Task {} status: running", task_id_clone);

                // Spawn event forwarding task
                let socket_for_events = socket_clone.clone();
                let task_id_for_events = task_id_clone.clone();
                let mut rx = client_rx;

                let event_handle = tokio::spawn(async move {
                    let mut message_counter = 0u64;
                    info!("Event forwarding task started for {}", task_id_for_events);

                    while let Ok(event) = rx.recv().await {
                        info!("Received OpenCode event: {}", event.event_type);

                        // Convert OpenCode events to task messages
                        let content = match event.event_type.as_str() {
                            // Text streaming from assistant
                            "message.part.delta" => {
                                // Extract text delta
                                event
                                    .properties
                                    .get("part")
                                    .and_then(|p| p.get("text"))
                                    .and_then(|t| t.as_str())
                                    .map(|s| s.to_string())
                            }
                            // Tool calls
                            "tool.start" => {
                                let tool_name = event
                                    .properties
                                    .get("tool")
                                    .and_then(|t| t.get("name"))
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("unknown");
                                Some(format!("[Tool] Starting: {}", tool_name))
                            }
                            "tool.end" => {
                                let tool_name = event
                                    .properties
                                    .get("tool")
                                    .and_then(|t| t.get("name"))
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("unknown");
                                Some(format!("[Tool] Completed: {}", tool_name))
                            }
                            // File operations
                            "file.write" | "file.edit" => {
                                let path = event
                                    .properties
                                    .get("path")
                                    .and_then(|p| p.as_str())
                                    .unwrap_or("unknown");
                                Some(format!("[File] Modified: {}", path))
                            }
                            "file.read" => {
                                let path = event
                                    .properties
                                    .get("path")
                                    .and_then(|p| p.as_str())
                                    .unwrap_or("unknown");
                                Some(format!("[File] Read: {}", path))
                            }
                            // Bash commands
                            "bash.start" => {
                                let cmd = event
                                    .properties
                                    .get("command")
                                    .and_then(|c| c.as_str())
                                    .unwrap_or("...");
                                Some(format!("[Bash] $ {}", cmd))
                            }
                            "bash.output" => event
                                .properties
                                .get("output")
                                .and_then(|o| o.as_str())
                                .map(|s| format!("{}", s)),
                            // Claude thinking
                            "assistant.thinking" => event
                                .properties
                                .get("thinking")
                                .and_then(|t| t.as_str())
                                .map(|s| format!("[Thinking] {}", s)),
                            // Message events - try to extract content
                            "message.created" => {
                                // Extract message content if available
                                event
                                    .properties
                                    .get("message")
                                    .and_then(|m| m.get("content"))
                                    .and_then(|c| {
                                        if let Some(arr) = c.as_array() {
                                            arr.iter()
                                                .filter_map(|part| {
                                                    part.get("text").and_then(|t| t.as_str())
                                                })
                                                .collect::<Vec<_>>()
                                                .join("")
                                                .into()
                                        } else {
                                            c.as_str().map(|s| s.to_string())
                                        }
                                    })
                                    .filter(|s| !s.is_empty())
                            }
                            "message.updated" => None,
                            // Session events
                            "session.created" => Some("Session created".to_string()),
                            "session.idle" => Some("Task completed".to_string()),
                            "session.error" => {
                                let error_msg = event
                                    .properties
                                    .get("error")
                                    .and_then(|e| e.as_str())
                                    .unwrap_or("Unknown error");
                                Some(format!("Error: {}", error_msg))
                            }
                            _ => {
                                // Log other event types for debugging
                                info!(
                                    "Unhandled event type: {} - {:?}",
                                    event.event_type, event.properties
                                );
                                None
                            }
                        };

                        if let Some(text) = content {
                            message_counter += 1;
                            info!(
                                "Sending task:message #{}: {}",
                                message_counter,
                                &text[..text.len().min(50)]
                            );
                            let msg = TaskMessagePayload {
                                task_id: task_id_for_events.clone(),
                                message: TaskMessage {
                                    id: format!("msg-{}", message_counter),
                                    role: "assistant".to_string(),
                                    content: text,
                                    timestamp: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis()
                                        as u64,
                                },
                            };
                            let _ = socket_for_events.emit("task:message", &msg);
                        }
                    }
                    info!("Event forwarding task ended for {}", task_id_for_events);
                });

                // Run the actual task
                let result = {
                    let session_guard = session.read().await;
                    session_guard.client.run(&prompt).await
                };

                // Wait a bit for remaining events
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                event_handle.abort();

                // Update status based on result
                match result {
                    Ok(_) => {
                        info!("Task {} completed successfully", task_id_clone);
                        let _ = socket_clone.emit(
                            "task:status",
                            &TaskStatusPayload {
                                task_id: task_id_clone.clone(),
                                status: "completed".to_string(),
                            },
                        );

                        // Move task to "done" column
                        if let Err(e) = kanban_store
                            .move_task(
                                &task_id_clone,
                                vk_core::kanban::KanbanTaskStatus::Done,
                                None,
                            )
                            .await
                        {
                            warn!("Failed to move task to done: {}", e);
                        } else {
                            // Broadcast kanban sync
                            let board_state = kanban_store.get_state().await;
                            let _ = socket_clone.emit("kanban:sync", &board_state);
                            let _ = socket_clone.broadcast().emit("kanban:sync", &board_state);
                        }
                    }
                    Err(e) => {
                        error!("Task {} failed: {}", task_id_clone, e);
                        let _ = socket_clone.emit(
                            "task:status",
                            &TaskStatusPayload {
                                task_id: task_id_clone.clone(),
                                status: "failed".to_string(),
                            },
                        );
                        let _ = socket_clone.emit(
                            "task:error",
                            &TaskErrorPayload {
                                task_id: task_id_clone.clone(),
                                error: e.to_string(),
                            },
                        );
                    }
                }

                // Cleanup session
                let session_guard = session.read().await;
                session_guard.client.stop().await;
                drop(session_guard);

                let mut sessions_guard = sessions.write().await;
                sessions_guard.remove(&task_id_clone);
            });
        },
    );

    // Task stop handler
    socket.on(
        "task:stop",
        |socket: SocketRef, State(state): State<SocketState>, Data(data): Data<TaskStopPayload>| async move {
            let task_id = data.task_id.clone();
            info!("Task stop requested: {}", task_id);

            let session_opt = {
                let sessions = state.sessions.read().await;
                sessions.get(&task_id).cloned()
            };

            match session_opt {
                Some(session) => {
                    let session_guard = session.read().await;
                    session_guard.client.stop().await;
                    drop(session_guard);

                    // Remove from sessions
                    let mut sessions = state.sessions.write().await;
                    sessions.remove(&task_id);

                    let _ = socket.emit("task:status", &TaskStatusPayload {
                        task_id: task_id.clone(),
                        status: "aborted".to_string(),
                    });
                }
                None => {
                    warn!("No running session for task: {}", task_id);
                    let _ = socket.emit("task:error", &TaskErrorPayload {
                        task_id: task_id.clone(),
                        error: "No running session for this task".to_string(),
                    });
                }
            }
        },
    );
}

async fn handle_request_sync(socket: SocketRef, state: SocketState) {
    info!("Client {} requested sync", socket.id);
    // Sync from TaskStore first to pick up any new tasks created via REST API
    match state.kanban_store.get_state_synced().await {
        Ok(board_state) => {
            if let Err(e) = socket.emit("kanban:sync", &board_state) {
                warn!("Failed to emit sync: {}", e);
            }
        }
        Err(e) => {
            warn!("Failed to sync state: {}", e);
            // Fall back to unsync'd state
            let board_state = state.kanban_store.get_state().await;
            let _ = socket.emit("kanban:sync", &board_state);
        }
    }
}

async fn handle_create_task(socket: SocketRef, state: SocketState, data: CreateTaskPayload) {
    info!("Creating task: {}", data.title);

    match state
        .kanban_store
        .create_task(&data.title, data.description.as_deref())
        .await
    {
        Ok(_task) => {
            // Broadcast updated state to all clients
            let board_state = state.kanban_store.get_state().await;
            broadcast_sync(&socket, &board_state);
        }
        Err(e) => {
            warn!("Failed to create task: {}", e);
            let _ = socket.emit(
                "kanban:error",
                &ErrorPayload {
                    message: e.to_string(),
                },
            );
        }
    }
}

async fn handle_move_task(socket: SocketRef, state: SocketState, data: MoveTaskPayload) {
    info!("Moving task {} to {:?}", data.task_id, data.target_status);

    match state
        .kanban_store
        .move_task(&data.task_id, data.target_status, data.target_index)
        .await
    {
        Ok(true) => {
            let board_state = state.kanban_store.get_state().await;
            broadcast_sync(&socket, &board_state);
        }
        Ok(false) => {
            let _ = socket.emit(
                "kanban:error",
                &ErrorPayload {
                    message: format!("Task not found: {}", data.task_id),
                },
            );
        }
        Err(e) => {
            warn!("Failed to move task: {}", e);
            let _ = socket.emit(
                "kanban:error",
                &ErrorPayload {
                    message: e.to_string(),
                },
            );
        }
    }
}

async fn handle_delete_task(socket: SocketRef, state: SocketState, data: DeleteTaskPayload) {
    info!("Deleting task: {}", data.task_id);

    // Delete from kanban store
    match state.kanban_store.delete_task(&data.task_id).await {
        Ok(Some(_)) => {
            // Also delete from task store to keep them in sync
            if let Ok(task_uuid) = uuid::Uuid::parse_str(&data.task_id) {
                if let Err(e) = state.task_store.delete(task_uuid).await {
                    warn!("Failed to delete from task store: {}", e);
                }
            }

            let board_state = state.kanban_store.get_state().await;
            broadcast_sync(&socket, &board_state);
        }
        Ok(None) => {
            let _ = socket.emit(
                "kanban:error",
                &ErrorPayload {
                    message: format!("Task not found: {}", data.task_id),
                },
            );
        }
        Err(e) => {
            warn!("Failed to delete task: {}", e);
            let _ = socket.emit(
                "kanban:error",
                &ErrorPayload {
                    message: e.to_string(),
                },
            );
        }
    }
}

/// Broadcast board state to all clients
fn broadcast_sync(socket: &SocketRef, board_state: &vk_core::kanban::KanbanBoardState) {
    // Emit to the sender
    let _ = socket.emit("kanban:sync", board_state);
    // Broadcast to all other clients
    let _ = socket.broadcast().emit("kanban:sync", board_state);
}

/// Create and configure Socket.IO layer
pub fn create_socket_layer(state: SocketState) -> (socketioxide::layer::SocketIoLayer, SocketIo) {
    let (layer, io) = SocketIo::builder()
        .with_state(state)
        // Only allow WebSocket transport to avoid CORS issues with polling
        .transports([TransportType::Websocket])
        .build_layer();

    io.ns("/", on_connect);

    (layer, io)
}
