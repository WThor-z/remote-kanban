//! WebSocket handler for Agent Gateway connections

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use super::manager::GatewayManager;
use super::protocol::*;

/// Query parameters for WebSocket connection
#[derive(Debug, serde::Deserialize)]
pub struct WsQuery {
    #[serde(rename = "hostId")]
    pub host_id: String,
}

/// WebSocket upgrade handler
pub async fn gateway_ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(manager): State<Arc<GatewayManager>>,
) -> impl IntoResponse {
    info!("New gateway connection request from host: {}", query.host_id);
    ws.on_upgrade(move |socket| handle_gateway_socket(socket, query.host_id, manager))
}

/// Handle an individual gateway WebSocket connection
async fn handle_gateway_socket(
    socket: WebSocket,
    host_id: String,
    manager: Arc<GatewayManager>,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    
    // Channel for sending messages to this gateway
    let (tx, mut rx) = mpsc::channel::<ServerToGatewayMessage>(100);

    // Task to forward messages from channel to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if ws_sender.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                }
            }
        }
    });

    let host_id_clone = host_id.clone();
    let manager_clone = Arc::clone(&manager);
    let tx_clone = tx.clone();

    // Process incoming messages from gateway
    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<GatewayToServerMessage>(&text) {
                    Ok(msg) => {
                        handle_gateway_message(
                            &manager_clone,
                            &host_id_clone,
                            msg,
                            tx_clone.clone(),
                        )
                        .await;
                    }
                    Err(e) => {
                        warn!("Failed to parse message from {}: {}", host_id_clone, e);
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("Gateway {} sent close frame", host_id_clone);
                break;
            }
            Ok(Message::Ping(data)) => {
                debug!("Ping from {}", host_id_clone);
                // Axum handles pong automatically for Message::Ping
                let _ = data; // Suppress unused warning
            }
            Ok(Message::Pong(_)) => {
                debug!("Pong from {}", host_id_clone);
            }
            Ok(Message::Binary(_)) => {
                warn!("Unexpected binary message from {}", host_id_clone);
            }
            Err(e) => {
                error!("WebSocket error from {}: {}", host_id_clone, e);
                break;
            }
        }
    }

    // Cleanup on disconnect
    info!("Gateway {} disconnected", host_id);
    manager.unregister_host(&host_id).await;
    send_task.abort();
}

/// Handle a single message from a gateway
async fn handle_gateway_message(
    manager: &GatewayManager,
    host_id: &str,
    msg: GatewayToServerMessage,
    tx: mpsc::Sender<ServerToGatewayMessage>,
) {
    match msg {
        GatewayToServerMessage::Register {
            host_id: msg_host_id,
            capabilities,
        } => {
            // Verify host_id matches (security check)
            if msg_host_id != host_id {
                warn!(
                    "Host ID mismatch: query={}, message={}",
                    host_id, msg_host_id
                );
            }
            
            let ok = manager
                .register_host(msg_host_id, capabilities, tx.clone())
                .await;
            
            let _ = tx
                .send(ServerToGatewayMessage::Registered { ok, error: None })
                .await;
        }

        GatewayToServerMessage::Heartbeat { timestamp: _ } => {
            manager.update_heartbeat(host_id).await;
        }

        GatewayToServerMessage::TaskStarted {
            task_id,
            session_id,
        } => {
            manager
                .handle_task_started(host_id, &task_id, &session_id)
                .await;
        }

        GatewayToServerMessage::TaskEvent { task_id, event } => {
            manager.handle_task_event(host_id, &task_id, event).await;
        }

        GatewayToServerMessage::TaskCompleted { task_id, result } => {
            manager
                .handle_task_completed(host_id, &task_id, result)
                .await;
        }

        GatewayToServerMessage::TaskFailed {
            task_id,
            error,
            details: _,
        } => {
            manager.handle_task_failed(host_id, &task_id, &error).await;
        }

        GatewayToServerMessage::ModelsResponse {
            request_id,
            providers,
        } => {
            manager.handle_models_response(&request_id, providers).await;
        }
    }
}

/// Start the heartbeat checker background task
pub fn start_heartbeat_checker(manager: Arc<GatewayManager>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            manager
                .cleanup_stale_connections(std::time::Duration::from_secs(90))
                .await;
        }
    });
}

/// List all connected hosts (for REST API)
pub async fn list_hosts_handler(
    State(manager): State<Arc<GatewayManager>>,
) -> impl IntoResponse {
    let hosts = manager.list_hosts().await;
    axum::Json(hosts)
}

/// Get available models from a specific gateway host
pub async fn get_host_models_handler(
    State(manager): State<Arc<GatewayManager>>,
    axum::extract::Path(host_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    match manager.request_models(&host_id).await {
        Ok(providers) => (axum::http::StatusCode::OK, axum::Json(providers)).into_response(),
        Err(e) => {
            warn!("Failed to get models from host {}: {}", host_id, e);
            (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                axum::Json(serde_json::json!({ "error": e })),
            )
                .into_response()
        }
    }
}
