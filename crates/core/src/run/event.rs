//! Event types for agent execution

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of an execution session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Initializing,
    CreatingWorktree,
    Starting,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
    CleaningUp,
}

impl ExecutionStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

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
    Thinking {
        content: String,
    },
    Command {
        command: String,
        output: String,
        exit_code: Option<i32>,
    },
    FileChange {
        path: String,
        action: FileAction,
        diff: Option<String>,
    },
    ToolCall {
        tool: String,
        args: serde_json::Value,
        result: Option<serde_json::Value>,
    },
    Message {
        content: String,
    },
    Error {
        message: String,
        recoverable: bool,
    },
    Completed {
        success: bool,
        summary: Option<String>,
    },
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
    pub id: Uuid,
    pub session_id: Uuid,
    pub task_id: Uuid,
    pub timestamp: DateTime<Utc>,
    #[serde(flatten)]
    pub event: ExecutionEventType,
}

/// Type of execution event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum ExecutionEventType {
    StatusChanged {
        old_status: ExecutionStatus,
        new_status: ExecutionStatus,
    },
    AgentEvent {
        #[serde(flatten)]
        event: AgentEvent,
    },
    SessionStarted {
        worktree_path: String,
        branch: String,
    },
    SessionEnded {
        status: ExecutionStatus,
        duration_ms: u64,
    },
    Progress {
        message: String,
        percentage: Option<f32>,
    },
}

impl ExecutionEvent {
    pub fn new(session_id: Uuid, task_id: Uuid, event: ExecutionEventType) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            task_id,
            timestamp: Utc::now(),
            event,
        }
    }

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

    pub fn agent_event(session_id: Uuid, task_id: Uuid, event: AgentEvent) -> Self {
        Self::new(
            session_id,
            task_id,
            ExecutionEventType::AgentEvent { event },
        )
    }

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
