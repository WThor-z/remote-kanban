//! Error types for the core library

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Agent error: {0}")]
    Agent(String),
}
