//! Agent module for AI-powered task execution
//!
//! This module provides integration with OpenCode CLI for executing tasks.

mod opencode_client;

pub use opencode_client::{OpencodeClient, OpencodeConfig, OpencodeEvent, OpencodeSession};
