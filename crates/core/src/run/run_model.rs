use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

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
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call: Option<ToolCallInfo>,
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
    pub id: Uuid,
    pub task_id: Uuid,
    pub agent_type: AgentType,
    pub prompt: String,
    pub base_branch: String,
    pub worktree_branch: Option<String>,
    pub worktree_path: Option<PathBuf>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub status: ExecutionStatus,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
    pub summary: Option<String>,
    pub events_path: Option<PathBuf>,
    pub event_count: u32,
    pub metadata: RunMetadata,
}

use super::agent_type::AgentType;
use super::event::ExecutionStatus;

/// Additional metadata for a run
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunMetadata {
    #[serde(default)]
    pub files_modified: Vec<String>,
    #[serde(default)]
    pub commands_executed: u32,
    #[serde(default)]
    pub tools_called: u32,
    #[serde(default)]
    pub thinking_count: u32,
    #[serde(default)]
    pub message_count: u32,
    #[serde(default)]
    pub error_count: u32,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Run {
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

    pub fn mark_started(&mut self) {
        self.started_at = Some(Utc::now());
        self.status = ExecutionStatus::Running;
    }

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

    pub fn mark_failed(&mut self, error: String) {
        self.ended_at = Some(Utc::now());
        self.error = Some(error);
        self.status = ExecutionStatus::Failed;
        self.calculate_duration();
    }

    pub fn mark_cancelled(&mut self) {
        self.ended_at = Some(Utc::now());
        self.status = ExecutionStatus::Cancelled;
        self.calculate_duration();
    }

    pub fn update_status(&mut self, status: ExecutionStatus) {
        self.status = status;
    }

    pub fn increment_event_count(&mut self) {
        self.event_count += 1;
    }

    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }

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
    pub id: Uuid,
    pub task_id: Uuid,
    pub agent_type: AgentType,
    pub prompt_preview: String,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub status: ExecutionStatus,
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
