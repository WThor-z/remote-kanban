//! Gateway routes for Agent Gateway WebSocket connections

use axum::{routing::get, Router};
use std::sync::Arc;

use crate::gateway::{gateway_ws_handler, list_hosts_handler, GatewayManager};

/// Create router for gateway endpoints
pub fn router(manager: Arc<GatewayManager>) -> Router<()> {
    Router::new()
        .route("/agent/ws", get(gateway_ws_handler))
        .route("/api/hosts", get(list_hosts_handler))
        .with_state(manager)
}
