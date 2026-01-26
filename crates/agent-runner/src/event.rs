//! Event types for agent execution

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of an execution session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// Session is being initialized
    Initializing,
    /// Worktree is being created
    CreatingWorktree,
    /// Agent process is starting
    Starting,
    /// Agent is running
    Running,
    /// Agent is paused (waiting for input)
    Paused,
    /// Execution completed successfully
    Completed,
    /// Execution failed
    Failed,
    /// Execution was cancelled
    Cancelled,
    /// Worktree is being cleaned up
    CleaningUp,
}

impl ExecutionStatus {
    /// Check if the status represents a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Check if the status represents an active state
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Initializing
                | Self::CreatingWorktree
                | Self::Starting
                | Self::Running
                | Self::Paused
                | Self::CleaningUp
        )
    }
}

/// Events emitted by the agent during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// Agent is thinking/processing
    Thinking { content: String },

    /// Agent executed a command
    Command {
        command: String,
        output: String,
        exit_code: Option<i32>,
    },

    /// Agent modified a file
    FileChange {
        path: String,
        action: FileAction,
        diff: Option<String>,
    },

    /// Agent called a tool
    ToolCall {
        tool: String,
        args: serde_json::Value,
        result: Option<serde_json::Value>,
    },

    /// Agent sent a message
    Message { content: String },

    /// Agent encountered an error
    Error { message: String, recoverable: bool },

    /// Agent completed the task
    Completed {
        success: bool,
        summary: Option<String>,
    },

    /// Raw output from the agent (for debugging)
    RawOutput {
        stream: OutputStream,
        content: String,
    },
}

/// Type of file action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileAction {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// Output stream type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// Execution event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    /// Unique event ID
    pub id: Uuid,

    /// Session ID this event belongs to
    pub session_id: Uuid,

    /// Task ID
    pub task_id: Uuid,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// The event payload
    #[serde(flatten)]
    pub event: ExecutionEventType,
}

/// Type of execution event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum ExecutionEventType {
    /// Status changed
    StatusChanged {
        old_status: ExecutionStatus,
        new_status: ExecutionStatus,
    },

    /// Agent event
    AgentEvent {
        #[serde(flatten)]
        event: AgentEvent,
    },

    /// Session started
    SessionStarted {
        worktree_path: String,
        branch: String,
    },

    /// Session ended
    SessionEnded {
        status: ExecutionStatus,
        duration_ms: u64,
    },

    /// Progress update
    Progress {
        message: String,
        percentage: Option<f32>,
    },
}

impl ExecutionEvent {
    /// Create a new execution event
    pub fn new(session_id: Uuid, task_id: Uuid, event: ExecutionEventType) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            task_id,
            timestamp: Utc::now(),
            event,
        }
    }

    /// Create a status changed event
    pub fn status_changed(
        session_id: Uuid,
        task_id: Uuid,
        old_status: ExecutionStatus,
        new_status: ExecutionStatus,
    ) -> Self {
        Self::new(
            session_id,
            task_id,
            ExecutionEventType::StatusChanged {
                old_status,
                new_status,
            },
        )
    }

    /// Create an agent event
    pub fn agent_event(session_id: Uuid, task_id: Uuid, event: AgentEvent) -> Self {
        Self::new(
            session_id,
            task_id,
            ExecutionEventType::AgentEvent { event },
        )
    }

    /// Create a session started event
    pub fn session_started(
        session_id: Uuid,
        task_id: Uuid,
        worktree_path: String,
        branch: String,
    ) -> Self {
        Self::new(
            session_id,
            task_id,
            ExecutionEventType::SessionStarted {
                worktree_path,
                branch,
            },
        )
    }

    /// Create a session ended event
    pub fn session_ended(
        session_id: Uuid,
        task_id: Uuid,
        status: ExecutionStatus,
        duration_ms: u64,
    ) -> Self {
        Self::new(
            session_id,
            task_id,
            ExecutionEventType::SessionEnded {
                status,
                duration_ms,
            },
        )
    }

    /// Create a progress event
    pub fn progress(
        session_id: Uuid,
        task_id: Uuid,
        message: String,
        percentage: Option<f32>,
    ) -> Self {
        Self::new(
            session_id,
            task_id,
            ExecutionEventType::Progress {
                message,
                percentage,
            },
        )
    }
}
