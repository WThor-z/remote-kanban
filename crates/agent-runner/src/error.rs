//! Error types for agent-runner

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for executor operations
pub type Result<T> = std::result::Result<T, ExecutorError>;

/// Errors that can occur during task execution
#[derive(Debug, Error)]
pub enum ExecutorError {
    /// Worktree operation failed
    #[error("Worktree error: {0}")]
    Worktree(#[from] git_worktree::WorktreeError),

    /// Failed to spawn agent process
    #[error("Failed to spawn agent process: {message}")]
    SpawnFailed {
        message: String,
        #[source]
        source: Option<std::io::Error>,
    },

    /// Agent process exited unexpectedly
    #[error("Agent process exited with code {code:?}: {message}")]
    ProcessExited { code: Option<i32>, message: String },

    /// Session not found
    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    /// Session already exists
    #[error("Session already exists for task: {task_id}")]
    SessionExists { task_id: String },

    /// Task not found
    #[error("Task not found: {task_id}")]
    TaskNotFound { task_id: String },

    /// Invalid agent type
    #[error("Invalid agent type: {agent_type}")]
    InvalidAgentType { agent_type: String },

    /// Execution timeout
    #[error("Execution timed out after {seconds} seconds")]
    Timeout { seconds: u64 },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Channel closed
    #[error("Event channel closed")]
    ChannelClosed,

    /// Worktree path not found
    #[error("Worktree path not found: {path}")]
    WorktreePathNotFound { path: PathBuf },

    /// Session is not running
    #[error("Session {session_id} is not running")]
    SessionNotRunning { session_id: String },

    /// Session is already running
    #[error("Session {session_id} is already running")]
    SessionAlreadyRunning { session_id: String },

    /// Execution failed
    #[error("Execution failed: {message}")]
    ExecutionFailed { message: String },
}

impl ExecutorError {
    /// Create a SpawnFailed error
    pub fn spawn_failed(message: impl Into<String>) -> Self {
        Self::SpawnFailed {
            message: message.into(),
            source: None,
        }
    }

    /// Create a SpawnFailed error with source
    pub fn spawn_failed_with_source(message: impl Into<String>, source: std::io::Error) -> Self {
        Self::SpawnFailed {
            message: message.into(),
            source: Some(source),
        }
    }

    /// Create an ExecutionFailed error
    pub fn execution_failed(message: impl Into<String>) -> Self {
        Self::ExecutionFailed {
            message: message.into(),
        }
    }
}
