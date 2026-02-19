//! WebSocket handler for Agent Gateway connections

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::IntoResponse,
    response::Response,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use super::manager::GatewayManager;
use super::protocol::*;
use crate::host::{HostStore, HostStoreError};

#[derive(Clone)]
pub struct GatewayRouteState {
    pub manager: Arc<GatewayManager>,
    pub host_store: Arc<HostStore>,
}

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
    State(state): State<GatewayRouteState>,
    headers: HeaderMap,
) -> Response {
    let token = match extract_bearer_token(&headers) {
        Ok(token) => token,
        Err(response) => return response,
    };
    let host_summary = match state
        .host_store
        .verify_connection_token(token, &query.host_id)
        .await
    {
        Ok(host_summary) => host_summary,
        Err(err) => {
            let status = map_host_auth_error(&err);
            warn!(
                "Rejected gateway connection host={} reason={} status={}",
                query.host_id, err, status
            );
            return (status, "Unauthorized").into_response();
        }
    };

    info!(
        "New gateway connection org={} host={}",
        host_summary.org_id, host_summary.host_id
    );
    ws.on_upgrade(move |socket| {
        handle_gateway_socket(
            socket,
            host_summary.org_id,
            host_summary.host_id,
            state.manager,
        )
    })
    .into_response()
}

/// Handle an individual gateway WebSocket connection
async fn handle_gateway_socket(
    socket: WebSocket,
    org_id: String,
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

    let org_id_clone = org_id.clone();
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
                            &org_id_clone,
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
    org_id: &str,
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
                .register_host_scoped(org_id.to_string(), msg_host_id, capabilities, tx.clone())
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

        GatewayToServerMessage::MemoryResponse {
            request_id,
            ok,
            data,
            error,
        } => {
            manager
                .handle_memory_response(&request_id, ok, data, error)
                .await;
        }

        GatewayToServerMessage::MemorySync { sync } => {
            manager.handle_memory_sync(sync).await;
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
pub async fn list_hosts_handler(State(state): State<GatewayRouteState>) -> impl IntoResponse {
    let hosts = state.manager.list_hosts().await;
    axum::Json(hosts)
}

/// Get available models from a specific gateway host
pub async fn get_host_models_handler(
    State(state): State<GatewayRouteState>,
    axum::extract::Path(host_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    match state.manager.request_models(&host_id).await {
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

fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, Response> {
    let auth_header = headers.get(AUTHORIZATION).ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "Missing Authorization header").into_response()
    })?;
    let auth_value = auth_header
        .to_str()
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid Authorization header").into_response())?;
    auth_value.strip_prefix("Bearer ").ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            "Authorization must be Bearer token",
        )
            .into_response()
    })
}

fn map_host_auth_error(err: &HostStoreError) -> StatusCode {
    match err {
        HostStoreError::InvalidInput(_) => StatusCode::BAD_REQUEST,
        HostStoreError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
        HostStoreError::Forbidden(_) => StatusCode::FORBIDDEN,
        HostStoreError::NotFound(_) => StatusCode::NOT_FOUND,
        HostStoreError::Conflict(_) => StatusCode::CONFLICT,
        HostStoreError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn extracts_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_static("Bearer secret-token"),
        );

        let token = extract_bearer_token(&headers).unwrap();
        assert_eq!(token, "secret-token");
    }

    #[test]
    fn rejects_when_authorization_header_missing() {
        let headers = HeaderMap::new();
        assert!(extract_bearer_token(&headers).is_err());
    }

    #[test]
    fn rejects_when_scheme_is_not_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_static("Basic secret-token"),
        );
        assert!(extract_bearer_token(&headers).is_err());
    }
}
