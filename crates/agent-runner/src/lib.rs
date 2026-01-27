//! Agent Runner - Task executor for isolated worktree execution
//!
//! This crate provides functionality for executing tasks in isolated
//! Git worktrees with agent processes (OpenCode, Claude Code, etc.)

mod error;
mod event;
mod executor;
mod process;
mod session;

pub use error::{ExecutorError, Result};
pub use event::{AgentEvent, ExecutionEvent, ExecutionStatus};
pub use executor::{ExecuteRequest, ExecutorConfig, TaskExecutor};
pub use process::AgentProcess;
pub use session::{ExecutionSession, SessionState};
