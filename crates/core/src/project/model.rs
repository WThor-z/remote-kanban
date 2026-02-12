//! Project model definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A Project represents a Git repository managed by a Gateway.
///
/// Each project is bound to exactly one Gateway (one-to-one relationship).
/// Tasks are created within a Project and use Git worktrees for isolation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique project identifier
    pub id: Uuid,

    /// Human-readable project name (e.g., "vibe-kanban")
    pub name: String,

    /// Local filesystem path on the Gateway machine
    pub local_path: String,

    /// Remote Git repository URL (e.g., "git@github.com:user/repo.git")
    /// Used for display and reference only
    pub remote_url: Option<String>,

    /// Default branch name (e.g., "main" or "master")
    pub default_branch: String,

    /// ID of the Gateway this project is bound to
    pub gateway_id: String,

    /// ID of the workspace this project belongs to
    pub workspace_id: Uuid,

    /// Directory name for worktrees (relative to project root)
    /// Default: ".worktrees"
    pub worktree_dir: String,

    /// Timestamp when the project was created
    pub created_at: DateTime<Utc>,

    /// Timestamp when the project was last updated
    pub updated_at: DateTime<Utc>,
}

impl Project {
    /// Create a new project with required fields
    pub fn new(
        name: impl Into<String>,
        local_path: impl Into<String>,
        gateway_id: impl Into<String>,
        workspace_id: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            local_path: local_path.into(),
            remote_url: None,
            default_branch: "main".to_string(),
            gateway_id: gateway_id.into(),
            workspace_id,
            worktree_dir: ".worktrees".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the remote URL
    pub fn with_remote_url(mut self, url: impl Into<String>) -> Self {
        self.remote_url = Some(url.into());
        self
    }

    /// Set the default branch
    pub fn with_default_branch(mut self, branch: impl Into<String>) -> Self {
        self.default_branch = branch.into();
        self
    }

    /// Set the worktree directory
    pub fn with_worktree_dir(mut self, dir: impl Into<String>) -> Self {
        self.worktree_dir = dir.into();
        self
    }

    /// Get the full path to the worktrees directory
    pub fn worktrees_path(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.local_path).join(&self.worktree_dir)
    }
}

/// Request to create or register a project (usually from Gateway)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    /// Project name
    pub name: String,

    /// Local path on Gateway machine
    pub local_path: String,

    /// Remote URL (optional)
    pub remote_url: Option<String>,

    /// Default branch (optional, defaults to "main")
    pub default_branch: Option<String>,

    /// Worktree directory (optional, defaults to ".worktrees")
    pub worktree_dir: Option<String>,

    /// Workspace that owns this project
    pub workspace_id: Uuid,
}

/// Summary view of a project for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: Uuid,
    pub name: String,
    pub local_path: String,
    pub remote_url: Option<String>,
    pub default_branch: String,
    pub gateway_id: String,
    pub workspace_id: Uuid,
    pub worktree_dir: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Project> for ProjectSummary {
    fn from(project: &Project) -> Self {
        Self {
            id: project.id,
            name: project.name.clone(),
            local_path: project.local_path.clone(),
            remote_url: project.remote_url.clone(),
            default_branch: project.default_branch.clone(),
            gateway_id: project.gateway_id.clone(),
            workspace_id: project.workspace_id,
            worktree_dir: project.worktree_dir.clone(),
            created_at: project.created_at,
            updated_at: project.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_project() {
        let gateway_id = "host-1".to_string();
        let workspace_id = Uuid::new_v4();
        let project = Project::new(
            "my-project",
            "/path/to/project",
            gateway_id.clone(),
            workspace_id,
        );

        assert_eq!(project.name, "my-project");
        assert_eq!(project.local_path, "/path/to/project");
        assert_eq!(project.default_branch, "main");
        assert_eq!(project.worktree_dir, ".worktrees");
        assert_eq!(project.gateway_id, gateway_id);
        assert_eq!(project.workspace_id, workspace_id);
        assert!(project.remote_url.is_none());
    }

    #[test]
    fn test_project_with_builders() {
        let gateway_id = "host-1".to_string();
        let workspace_id = Uuid::new_v4();
        let project = Project::new("my-project", "/path/to/project", gateway_id, workspace_id)
            .with_remote_url("git@github.com:user/repo.git")
            .with_default_branch("master")
            .with_worktree_dir(".git-worktrees");

        assert_eq!(
            project.remote_url,
            Some("git@github.com:user/repo.git".to_string())
        );
        assert_eq!(project.default_branch, "master");
        assert_eq!(project.worktree_dir, ".git-worktrees");
    }

    #[test]
    fn test_worktrees_path() {
        let gateway_id = "host-1".to_string();
        let workspace_id = Uuid::new_v4();
        let project = Project::new("my-project", "/path/to/project", gateway_id, workspace_id);

        let expected = std::path::PathBuf::from("/path/to/project/.worktrees");
        assert_eq!(project.worktrees_path(), expected);
    }

    #[test]
    fn test_project_summary_includes_workspace_id() {
        let gateway_id = "host-1".to_string();
        let workspace_id = Uuid::new_v4();
        let project = Project::new("my-project", "/path/to/project", gateway_id, workspace_id);

        let summary = ProjectSummary::from(&project);
        assert_eq!(summary.workspace_id, workspace_id);
    }
}
