//! Gateway routes for Agent Gateway WebSocket connections

use axum::{routing::get, Router};

use crate::gateway::{gateway_ws_handler, get_host_models_handler, list_hosts_handler};
use crate::state::AppState;

/// Create router for gateway endpoints
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/agent/ws", get(gateway_ws_handler))
        .route("/api/hosts", get(list_hosts_handler))
        .route("/api/hosts/{host_id}/models", get(get_host_models_handler))
}
