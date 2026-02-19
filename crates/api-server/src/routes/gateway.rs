//! Gateway routes for Agent Gateway WebSocket connections

use axum::{routing::get, Router};
use std::sync::Arc;

use crate::gateway::{
    gateway_ws_handler, get_host_models_handler, list_hosts_handler, GatewayManager,
    GatewayRouteState,
};
use crate::host::HostStore;

/// Create router for gateway endpoints
pub fn router(manager: Arc<GatewayManager>, host_store: Arc<HostStore>) -> Router<()> {
    let state = GatewayRouteState {
        manager,
        host_store,
    };
    Router::new()
        .route("/agent/ws", get(gateway_ws_handler))
        .route("/api/hosts", get(list_hosts_handler))
        .route("/api/hosts/{host_id}/models", get(get_host_models_handler))
        .with_state(state)
}
