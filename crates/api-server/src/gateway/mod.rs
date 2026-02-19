//! Gateway module for Agent Gateway communication
//!
//! This module provides WebSocket-based communication with remote Agent Gateways
//! that execute tasks on behalf of the server.

pub mod handler;
pub mod manager;
pub mod protocol;

pub use handler::{
    gateway_ws_handler, get_host_models_handler, list_hosts_handler, start_heartbeat_checker,
    GatewayRouteState,
};
pub use manager::GatewayManager;
