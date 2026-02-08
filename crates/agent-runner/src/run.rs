//! Run - Persistent execution record
//!
//! A Run represents a complete execution record for a task.
//! Unlike ExecutionSession (in-memory), Run is designed for persistence
//! and can be serialized to/from disk.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::event::ExecutionStatus;
use crate::process::AgentType;

/// Message role in a chat
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// A persisted chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Unique message ID
    pub id: String,

    /// Role of the message sender
    pub role: MessageRole,

    /// Message content
    pub content: String,

    /// Timestamp in milliseconds
    pub timestamp: i64,

    /// Message type (text, tool_use, thinking, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_type: Option<String>,

    /// Tool call information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call: Option<ToolCallInfo>,

    /// Tool result information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<ToolResultInfo>,
}

/// Tool call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub name: String,
    #[serde(default)]
    pub input: serde_json::Value,
}

/// Tool result information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultInfo {
    pub success: bool,
    pub output: String,
}

impl ChatMessage {
    /// Create a new user message
    pub fn user(content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content,
            timestamp: Utc::now().timestamp_millis(),
            message_type: Some("text".to_string()),
            tool_call: None,
            tool_result: None,
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            content,
            timestamp: Utc::now().timestamp_millis(),
            message_type: Some("text".to_string()),
            tool_call: None,
            tool_result: None,
        }
    }

    /// Create a new system message
    pub fn system(content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::System,
            content,
            timestamp: Utc::now().timestamp_millis(),
            message_type: Some("system".to_string()),
            tool_call: None,
            tool_result: None,
        }
    }

    /// Create with a specific ID
    pub fn with_id(id: String, role: MessageRole, content: String) -> Self {
        Self {
            id,
            role,
            content,
            timestamp: Utc::now().timestamp_millis(),
            message_type: Some("text".to_string()),
            tool_call: None,
            tool_result: None,
        }
    }
}

/// A persistent execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    /// Unique run ID (same as session ID during execution)
    pub id: Uuid,

    /// Task ID this run belongs to
    pub task_id: Uuid,

    /// Agent type used for execution
    pub agent_type: AgentType,

    /// The prompt that was executed
    pub prompt: String,

    /// Base branch used
    pub base_branch: String,

    /// Worktree branch created for this run
    pub worktree_branch: Option<String>,

    /// Worktree path (relative to repo root)
    pub worktree_path: Option<PathBuf>,

    /// When the run was created
    pub created_at: DateTime<Utc>,

    /// When execution started
    pub started_at: Option<DateTime<Utc>>,

    /// When execution ended
    pub ended_at: Option<DateTime<Utc>>,

    /// Duration in milliseconds
    pub duration_ms: Option<u64>,

    /// Final status
    pub status: ExecutionStatus,

    /// Exit code (if completed)
    pub exit_code: Option<i32>,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Summary of what was accomplished
    pub summary: Option<String>,

    /// Path to events log file (relative to data dir)
    pub events_path: Option<PathBuf>,

    /// Number of events in the log
    pub event_count: u32,

    /// Run metadata
    pub metadata: RunMetadata,
}

/// Additional metadata for a run
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunMetadata {
    /// Files modified during this run
    #[serde(default)]
    pub files_modified: Vec<String>,

    /// Commands executed during this run
    #[serde(default)]
    pub commands_executed: u32,

    /// Tools called during this run
    #[serde(default)]
    pub tools_called: u32,

    /// Number of thinking events
    #[serde(default)]
    pub thinking_count: u32,

    /// Number of messages
    #[serde(default)]
    pub message_count: u32,

    /// Number of errors (including recoverable)
    #[serde(default)]
    pub error_count: u32,

    /// Custom tags
    #[serde(default)]
    pub tags: Vec<String>,

    /// Bound project context for this run
    #[serde(default)]
    pub project_id: Option<Uuid>,

    /// Bound workspace context for this run
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
}

impl Run {
    /// Create a new run from basic info
    pub fn new(task_id: Uuid, agent_type: AgentType, prompt: String, base_branch: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_id,
            agent_type,
            prompt,
            base_branch,
            worktree_branch: None,
            worktree_path: None,
            created_at: Utc::now(),
            started_at: None,
            ended_at: None,
            duration_ms: None,
            status: ExecutionStatus::Initializing,
            exit_code: None,
            error: None,
            summary: None,
            events_path: None,
            event_count: 0,
            metadata: RunMetadata::default(),
        }
    }

    /// Create a run with a specific ID
    pub fn with_id(
        id: Uuid,
        task_id: Uuid,
        agent_type: AgentType,
        prompt: String,
        base_branch: String,
    ) -> Self {
        let mut run = Self::new(task_id, agent_type, prompt, base_branch);
        run.id = id;
        run
    }

    /// Mark the run as started
    pub fn mark_started(&mut self) {
        self.started_at = Some(Utc::now());
        self.status = ExecutionStatus::Running;
    }

    /// Mark the run as completed
    pub fn mark_completed(&mut self, exit_code: i32, summary: Option<String>) {
        let now = Utc::now();
        self.ended_at = Some(now);
        self.exit_code = Some(exit_code);
        self.summary = summary;
        self.status = if exit_code == 0 {
            ExecutionStatus::Completed
        } else {
            ExecutionStatus::Failed
        };
        self.calculate_duration();
    }

    /// Mark the run as failed
    pub fn mark_failed(&mut self, error: String) {
        self.ended_at = Some(Utc::now());
        self.error = Some(error);
        self.status = ExecutionStatus::Failed;
        self.calculate_duration();
    }

    /// Mark the run as cancelled
    pub fn mark_cancelled(&mut self) {
        self.ended_at = Some(Utc::now());
        self.status = ExecutionStatus::Cancelled;
        self.calculate_duration();
    }

    /// Update status
    pub fn update_status(&mut self, status: ExecutionStatus) {
        self.status = status;
    }

    /// Increment event count
    pub fn increment_event_count(&mut self) {
        self.event_count += 1;
    }

    /// Check if the run is in a terminal state
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    /// Check if the run is active
    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }

    /// Calculate duration from started_at to ended_at
    fn calculate_duration(&mut self) {
        if let (Some(started), Some(ended)) = (self.started_at, self.ended_at) {
            let duration = ended.signed_duration_since(started);
            self.duration_ms = Some(duration.num_milliseconds().max(0) as u64);
        }
    }
}

/// Summary of a run for listing purposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    /// Run ID
    pub id: Uuid,

    /// Task ID
    pub task_id: Uuid,

    /// Agent type
    pub agent_type: AgentType,

    /// Truncated prompt (first 100 chars)
    pub prompt_preview: String,

    /// When created
    pub created_at: DateTime<Utc>,

    /// When started
    pub started_at: Option<DateTime<Utc>>,

    /// When ended
    pub ended_at: Option<DateTime<Utc>>,

    /// Duration in milliseconds
    pub duration_ms: Option<u64>,

    /// Final status
    pub status: ExecutionStatus,

    /// Event count
    pub event_count: u32,
}

impl From<&Run> for RunSummary {
    fn from(run: &Run) -> Self {
        let prompt_preview = if run.prompt.len() > 100 {
            format!("{}...", &run.prompt[..100])
        } else {
            run.prompt.clone()
        };

        Self {
            id: run.id,
            task_id: run.task_id,
            agent_type: run.agent_type.clone(),
            prompt_preview,
            created_at: run.created_at,
            started_at: run.started_at,
            ended_at: run.ended_at,
            duration_ms: run.duration_ms,
            status: run.status,
            event_count: run.event_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_creation() {
        let task_id = Uuid::new_v4();
        let run = Run::new(
            task_id,
            AgentType::OpenCode,
            "Test prompt".to_string(),
            "main".to_string(),
        );

        assert!(!run.id.is_nil());
        assert_eq!(run.task_id, task_id);
        assert_eq!(run.status, ExecutionStatus::Initializing);
        assert!(run.started_at.is_none());
        assert!(run.ended_at.is_none());
    }

    #[test]
    fn test_run_lifecycle() {
        let mut run = Run::new(
            Uuid::new_v4(),
            AgentType::OpenCode,
            "Test".to_string(),
            "main".to_string(),
        );

        // Start
        run.mark_started();
        assert_eq!(run.status, ExecutionStatus::Running);
        assert!(run.started_at.is_some());

        // Complete
        run.mark_completed(0, Some("Done".to_string()));
        assert_eq!(run.status, ExecutionStatus::Completed);
        assert!(run.ended_at.is_some());
        assert_eq!(run.exit_code, Some(0));
        assert!(run.duration_ms.is_some());
    }

    #[test]
    fn test_run_failure() {
        let mut run = Run::new(
            Uuid::new_v4(),
            AgentType::OpenCode,
            "Test".to_string(),
            "main".to_string(),
        );

        run.mark_started();
        run.mark_failed("Something went wrong".to_string());

        assert_eq!(run.status, ExecutionStatus::Failed);
        assert_eq!(run.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_run_summary() {
        let run = Run::new(
            Uuid::new_v4(),
            AgentType::OpenCode,
            "A very long prompt that should be truncated when converted to summary for display in lists".to_string(),
            "main".to_string(),
        );

        let summary = RunSummary::from(&run);
        assert!(summary.prompt_preview.len() <= 103); // 100 + "..."
    }

    #[test]
    fn test_run_metadata() {
        let mut run = Run::new(
            Uuid::new_v4(),
            AgentType::OpenCode,
            "Test".to_string(),
            "main".to_string(),
        );
        let project_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        run.metadata.files_modified.push("src/main.rs".to_string());
        run.metadata.commands_executed = 5;
        run.metadata.tools_called = 3;
        run.metadata.project_id = Some(project_id);
        run.metadata.workspace_id = Some(workspace_id);

        assert_eq!(run.metadata.files_modified.len(), 1);
        assert_eq!(run.metadata.commands_executed, 5);
        assert_eq!(run.metadata.project_id, Some(project_id));
        assert_eq!(run.metadata.workspace_id, Some(workspace_id));
    }

    #[test]
    fn test_run_context_defaults_to_none() {
        let run = Run::new(
            Uuid::new_v4(),
            AgentType::OpenCode,
            "Test".to_string(),
            "main".to_string(),
        );

        assert_eq!(run.metadata.project_id, None);
        assert_eq!(run.metadata.workspace_id, None);
    }
}
