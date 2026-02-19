//! Task model definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Task status in the kanban board
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    InReview,
    Done,
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::Todo
    }
}

/// Task priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Medium
    }
}

/// A task in the kanban board
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    #[serde(default = "default_org_id")]
    pub org_id: String,
    pub project_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub agent_type: Option<String>,
    pub base_branch: Option<String>,
    pub model: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    /// Create a new task with the given title
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            org_id: default_org_id(),
            project_id: None,
            workspace_id: None,
            title: title.into(),
            description: None,
            status: TaskStatus::default(),
            priority: TaskPriority::default(),
            agent_type: Some("opencode".to_string()),
            base_branch: Some("main".to_string()),
            model: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the project id
    pub fn with_project_id(mut self, project_id: Uuid) -> Self {
        self.project_id = Some(project_id);
        self.workspace_id = None;
        self
    }

    /// Set the workspace id
    pub fn with_workspace_id(mut self, workspace_id: Uuid) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }

    /// Set both project and workspace bindings
    pub fn with_project_binding(mut self, project_id: Uuid, workspace_id: Uuid) -> Self {
        self.project_id = Some(project_id);
        self.workspace_id = Some(workspace_id);
        self
    }

    /// Set organization id
    pub fn with_org_id(mut self, org_id: impl Into<String>) -> Self {
        let normalized = org_id.into().trim().to_string();
        if !normalized.is_empty() {
            self.org_id = normalized;
        }
        self
    }

    /// Set the priority
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set the agent type
    pub fn with_agent_type(mut self, agent_type: impl Into<String>) -> Self {
        self.agent_type = Some(agent_type.into());
        self
    }

    /// Set the base branch
    pub fn with_base_branch(mut self, base_branch: impl Into<String>) -> Self {
        self.base_branch = Some(base_branch.into());
        self
    }

    /// Set the model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

fn default_org_id() -> String {
    std::env::var("VK_DEFAULT_ORG_ID")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "default-org".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_task() {
        let task = Task::new("Test task");
        assert_eq!(task.title, "Test task");
        assert!(!task.org_id.is_empty());
        assert!(task.project_id.is_none());
        assert!(task.workspace_id.is_none());
        assert_eq!(task.status, TaskStatus::Todo);
        assert_eq!(task.priority, TaskPriority::Medium);
        assert!(task.description.is_none());
    }

    #[test]
    fn test_task_with_project_id() {
        let project_id = Uuid::new_v4();
        let task = Task::new("Test task").with_project_id(project_id);

        assert_eq!(task.project_id, Some(project_id));
    }

    #[test]
    fn test_task_with_workspace_id() {
        let workspace_id = Uuid::new_v4();
        let task = Task::new("Test task").with_workspace_id(workspace_id);

        assert_eq!(task.workspace_id, Some(workspace_id));
    }

    #[test]
    fn test_task_with_project_binding() {
        let project_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let task = Task::new("Test task").with_project_binding(project_id, workspace_id);

        assert_eq!(task.project_id, Some(project_id));
        assert_eq!(task.workspace_id, Some(workspace_id));
    }

    #[test]
    fn test_task_with_org_id() {
        let task = Task::new("Test task").with_org_id("org-test");
        assert_eq!(task.org_id, "org-test");
    }

    #[test]
    fn test_task_with_project_id_clears_existing_workspace_binding() {
        let project_id_a = Uuid::new_v4();
        let project_id_b = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        let task = Task::new("Test task")
            .with_project_binding(project_id_a, workspace_id)
            .with_project_id(project_id_b);

        assert_eq!(task.project_id, Some(project_id_b));
        assert!(task.workspace_id.is_none());
    }

    #[test]
    fn test_task_with_description() {
        let task = Task::new("Test task").with_description("This is a test");
        assert_eq!(task.description, Some("This is a test".to_string()));
    }

    #[test]
    fn test_task_with_priority() {
        let task = Task::new("Test task").with_priority(TaskPriority::High);
        assert_eq!(task.priority, TaskPriority::High);
    }
}
