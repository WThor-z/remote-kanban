//! Gateway module for Agent Gateway communication
//!
//! This module provides WebSocket-based communication with remote Agent Gateways
//! that execute tasks on behalf of the server.

pub mod protocol;

pub use protocol::*;
