//! Error types for git-worktree operations

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for worktree operations
pub type Result<T> = std::result::Result<T, WorktreeError>;

/// Errors that can occur during worktree operations
#[derive(Debug, Error)]
pub enum WorktreeError {
    /// Git command execution failed
    #[error("Git command failed: {message}")]
    GitCommandFailed {
        message: String,
        #[source]
        source: Option<std::io::Error>,
    },

    /// Worktree already exists
    #[error("Worktree already exists at {path}")]
    WorktreeExists { path: PathBuf },

    /// Worktree not found
    #[error("Worktree not found at {path}")]
    WorktreeNotFound { path: PathBuf },

    /// Branch already exists
    #[error("Branch '{branch}' already exists")]
    BranchExists { branch: String },

    /// Branch not found
    #[error("Branch '{branch}' not found")]
    BranchNotFound { branch: String },

    /// Not a git repository
    #[error("Not a git repository: {path}")]
    NotAGitRepository { path: PathBuf },

    /// Invalid worktree path
    #[error("Invalid worktree path: {path}")]
    InvalidPath { path: PathBuf },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse git output
    #[error("Failed to parse git output: {message}")]
    ParseError { message: String },

    /// Worktree is locked
    #[error("Worktree at {path} is locked: {reason}")]
    WorktreeLocked { path: PathBuf, reason: String },
}

impl WorktreeError {
    /// Create a GitCommandFailed error
    pub fn git_failed(message: impl Into<String>) -> Self {
        Self::GitCommandFailed {
            message: message.into(),
            source: None,
        }
    }

    /// Create a GitCommandFailed error with source
    pub fn git_failed_with_source(message: impl Into<String>, source: std::io::Error) -> Self {
        Self::GitCommandFailed {
            message: message.into(),
            source: Some(source),
        }
    }
}
