//! Socket.IO event handlers for real-time communication
//!
//! This module provides Socket.IO integration for the kanban board,
//! compatible with the existing frontend.

use serde::{Deserialize, Serialize};
use socketioxide::extract::{Data, SocketRef, State};
use socketioxide::{SocketIo, TransportType};
use std::sync::Arc;
use tracing::{info, warn};

use vk_core::kanban::{KanbanStore, KanbanTaskStatus};
use vk_core::task::{FileTaskStore, TaskRepository};

/// Shared state for Socket.IO handlers
#[derive(Clone)]
pub struct SocketState {
    pub kanban_store: Arc<KanbanStore>,
    pub task_store: Arc<FileTaskStore>,
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
        |socket: SocketRef, State(state): State<SocketState>, Data(data): Data<CreateTaskPayload>| async move {
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
        |socket: SocketRef, State(state): State<SocketState>, Data(data): Data<DeleteTaskPayload>| async move {
            handle_delete_task(socket, state, data).await;
        },
    );

    socket.on_disconnect(|socket: SocketRef| async move {
        info!("Client disconnected: {}", socket.id);
    });

    // Task events (stub implementations for M-1.4)
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
        |socket: SocketRef, Data(data): Data<TaskHistoryPayload>| async move {
            warn!("Task execution not yet implemented: {}", data.task_id);
            let _ = socket.emit("task:error", &ErrorPayload {
                message: "Task execution not yet implemented. Coming in M-1.5!".to_string(),
            });
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
            let _ = socket.emit("kanban:error", &ErrorPayload { message: e.to_string() });
        }
    }
}

async fn handle_move_task(socket: SocketRef, state: SocketState, data: MoveTaskPayload) {
    info!(
        "Moving task {} to {:?}",
        data.task_id, data.target_status
    );

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
            let _ = socket.emit("kanban:error", &ErrorPayload { message: e.to_string() });
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
            let _ = socket.emit("kanban:error", &ErrorPayload { message: e.to_string() });
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
