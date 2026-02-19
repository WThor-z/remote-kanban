//! Agent Runner - Task executor for isolated worktree execution
//!
//! This crate provides functionality for executing tasks in isolated
//! Git worktrees with agent processes (OpenCode, Claude Code, etc.)

mod client;
mod error;
mod event;
mod executor;
mod parser;
mod persistence;
mod process;
mod run;
mod session;

pub use client::WorkerClient;
pub use error::{ExecutorError, Result};
pub use event::{AgentEvent, ExecutionEvent, ExecutionEventType, ExecutionStatus};
pub use executor::{ExecuteRequest, ExecutorConfig, TaskExecutor};
pub use persistence::RunStore;
pub use process::{AgentProcess, AgentType};
pub use run::{
    ChatMessage, MessageRole, Run, RunMetadata, RunSummary, ToolCallInfo, ToolResultInfo,
};
pub use session::{ExecutionSession, SessionState};
