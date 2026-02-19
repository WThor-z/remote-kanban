//! Project persistent store
//!
//! Provides file-based persistence for projects.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::Error;
use crate::Result;

use super::model::{CreateProjectRequest, Project, ProjectSummary};

fn default_org_id() -> String {
    std::env::var("VK_DEFAULT_ORG_ID")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "default-org".to_string())
}

/// Thread-safe project store with file persistence
#[derive(Clone)]
pub struct ProjectStore {
    /// In-memory cache of projects
    projects: Arc<RwLock<HashMap<Uuid, Project>>>,
    /// Path to the projects JSON file
    file_path: PathBuf,
}

impl ProjectStore {
    /// Create a new ProjectStore with the given file path
    pub async fn new(file_path: PathBuf, default_workspace_id: Uuid) -> Result<Self> {
        let (projects, migrated_legacy_projects) = if file_path.exists() {
            let content = tokio::fs::read_to_string(&file_path)
                .await
                .map_err(|e| Error::Storage(format!("Failed to read projects file: {}", e)))?;
            let loaded: HashMap<Uuid, StoredProject> = serde_json::from_str(&content)
                .map_err(|e| Error::Storage(format!("Failed to parse projects file: {}", e)))?;

            let mut migrated = false;
            let projects = loaded
                .into_values()
                .map(|project| {
                    let workspace_id = match project.workspace_id {
                        Some(workspace_id) => workspace_id,
                        None => {
                            migrated = true;
                            default_workspace_id
                        }
                    };
                    let org_id = match project.org_id {
                        Some(org_id) if !org_id.trim().is_empty() => org_id.trim().to_string(),
                        _ => {
                            migrated = true;
                            default_org_id()
                        }
                    };
                    (
                        project.id,
                        Project {
                            id: project.id,
                            name: project.name,
                            org_id,
                            local_path: project.local_path,
                            remote_url: project.remote_url,
                            default_branch: project.default_branch,
                            gateway_id: project.gateway_id,
                            workspace_id,
                            worktree_dir: project.worktree_dir,
                            created_at: project.created_at,
                            updated_at: project.updated_at,
                        },
                    )
                })
                .collect();
            (projects, migrated)
        } else {
            (HashMap::new(), false)
        };

        let store = Self {
            projects: Arc::new(RwLock::new(projects)),
            file_path,
        };

        if migrated_legacy_projects {
            store.persist().await?;
        }

        Ok(store)
    }

    /// Create or update a project from a Gateway registration request
    ///
    /// If a project with the same (gateway_id, local_path) exists, update it.
    /// Otherwise, create a new project.
    pub async fn register(
        &self,
        gateway_id: String,
        request: CreateProjectRequest,
    ) -> Result<Project> {
        let mut projects = self.projects.write().await;
        let request_org_id = request
            .org_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(default_org_id);

        // Check if project already exists for this gateway + path
        let existing = projects
            .values()
            .find(|p| p.gateway_id == gateway_id && p.local_path == request.local_path);

        let project = if let Some(existing) = existing {
            // Update existing project
            let mut updated = existing.clone();
            updated.name = request.name;
            updated.org_id = request_org_id.clone();
            updated.remote_url = request.remote_url;
            updated.workspace_id = request.workspace_id;
            if let Some(branch) = request.default_branch {
                updated.default_branch = branch;
            }
            if let Some(dir) = request.worktree_dir {
                updated.worktree_dir = dir;
            }
            updated.updated_at = chrono::Utc::now();
            projects.insert(updated.id, updated.clone());
            updated
        } else {
            // Create new project
            let mut project = Project::new(
                request.name,
                request.local_path,
                gateway_id,
                request.workspace_id,
            )
            .with_org_id(request_org_id);
            if let Some(url) = request.remote_url {
                project = project.with_remote_url(url);
            }
            if let Some(branch) = request.default_branch {
                project = project.with_default_branch(branch);
            }
            if let Some(dir) = request.worktree_dir {
                project = project.with_worktree_dir(dir);
            }
            projects.insert(project.id, project.clone());
            project
        };

        drop(projects);
        self.persist().await?;
        Ok(project)
    }

    /// Get a project by ID
    pub async fn get(&self, id: Uuid) -> Option<Project> {
        let projects = self.projects.read().await;
        projects.get(&id).cloned()
    }

    /// Get a project by gateway_id and local_path
    pub async fn get_by_gateway_path(&self, gateway_id: &str, local_path: &str) -> Option<Project> {
        let projects = self.projects.read().await;
        projects
            .values()
            .find(|p| p.gateway_id == gateway_id && p.local_path == local_path)
            .cloned()
    }

    /// List all projects
    pub async fn list(&self) -> Vec<ProjectSummary> {
        let projects = self.projects.read().await;
        projects.values().map(ProjectSummary::from).collect()
    }

    /// List projects for a specific gateway
    pub async fn list_by_gateway(&self, gateway_id: &str) -> Vec<ProjectSummary> {
        let projects = self.projects.read().await;
        projects
            .values()
            .filter(|p| p.gateway_id == gateway_id)
            .map(ProjectSummary::from)
            .collect()
    }

    /// Update a project
    pub async fn update(&self, project: Project) -> Result<Project> {
        let mut projects = self.projects.write().await;

        if !projects.contains_key(&project.id) {
            return Err(Error::NotFound(format!("Project {} not found", project.id)));
        }

        let mut updated = project;
        updated.updated_at = chrono::Utc::now();
        projects.insert(updated.id, updated.clone());

        drop(projects);
        self.persist().await?;
        Ok(updated)
    }

    /// Delete a project
    pub async fn delete(&self, id: Uuid) -> Result<Option<Project>> {
        let mut projects = self.projects.write().await;
        let removed = projects.remove(&id);

        if removed.is_some() {
            drop(projects);
            self.persist().await?;
        }

        Ok(removed)
    }

    /// Persist the current state to file
    async fn persist(&self) -> Result<()> {
        let projects = self.projects.read().await;
        let content = serde_json::to_string_pretty(&*projects)
            .map_err(|e| Error::Storage(format!("Failed to serialize projects: {}", e)))?;

        // Ensure parent directory exists
        if let Some(parent) = self.file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::Storage(format!("Failed to create directory: {}", e)))?;
        }

        tokio::fs::write(&self.file_path, content)
            .await
            .map_err(|e| Error::Storage(format!("Failed to write projects file: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_project_store() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("projects.json");
        let default_workspace_id = Uuid::new_v4();

        let store = ProjectStore::new(path, default_workspace_id).await.unwrap();
        let projects = store.list().await;

        assert_eq!(projects.len(), 0);
    }

    #[tokio::test]
    async fn test_register_project() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("projects.json");
        let default_workspace_id = Uuid::new_v4();

        let store = ProjectStore::new(path.clone(), default_workspace_id)
            .await
            .unwrap();
        let gateway_id = "host-one".to_string();
        let workspace_id = Uuid::new_v4();

        let request = CreateProjectRequest {
            name: "test-project".to_string(),
            local_path: "/path/to/project".to_string(),
            remote_url: Some("git@github.com:user/repo.git".to_string()),
            default_branch: Some("main".to_string()),
            worktree_dir: None,
            workspace_id,
            org_id: Some("org-test".to_string()),
        };

        let project = store.register(gateway_id.clone(), request).await.unwrap();

        assert_eq!(project.name, "test-project");
        assert_eq!(project.gateway_id, gateway_id);
        assert_eq!(project.workspace_id, workspace_id);
        assert_eq!(project.org_id, "org-test");

        // Verify persistence
        let store2 = ProjectStore::new(path, default_workspace_id).await.unwrap();
        let projects = store2.list().await;
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].workspace_id, workspace_id);
        assert_eq!(projects[0].org_id, "org-test");
    }

    #[tokio::test]
    async fn test_register_project_updates_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("projects.json");
        let default_workspace_id = Uuid::new_v4();

        let store = ProjectStore::new(path, default_workspace_id).await.unwrap();
        let gateway_id = "host-one".to_string();
        let workspace_id = Uuid::new_v4();

        let request1 = CreateProjectRequest {
            name: "project-v1".to_string(),
            local_path: "/path/to/project".to_string(),
            remote_url: None,
            default_branch: None,
            worktree_dir: None,
            workspace_id,
            org_id: Some("org-one".to_string()),
        };

        let project1 = store.register(gateway_id.clone(), request1).await.unwrap();

        // Register again with same gateway + path
        let request2 = CreateProjectRequest {
            name: "project-v2".to_string(),
            local_path: "/path/to/project".to_string(),
            remote_url: Some("git@github.com:user/repo.git".to_string()),
            default_branch: None,
            worktree_dir: None,
            workspace_id,
            org_id: Some("org-two".to_string()),
        };

        let project2 = store.register(gateway_id.clone(), request2).await.unwrap();

        // Should update, not create new
        assert_eq!(project1.id, project2.id);
        assert_eq!(project2.name, "project-v2");
        assert!(project2.remote_url.is_some());
        assert_eq!(project2.workspace_id, workspace_id);
        assert_eq!(project2.org_id, "org-two");

        let projects = store.list().await;
        assert_eq!(projects.len(), 1);
    }

    #[tokio::test]
    async fn test_register_updates_existing_workspace_id() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("projects.json");
        let default_workspace_id = Uuid::new_v4();

        let store = ProjectStore::new(path, default_workspace_id).await.unwrap();
        let gateway_id = "host-one".to_string();
        let workspace_id_1 = Uuid::new_v4();
        let workspace_id_2 = Uuid::new_v4();

        let first = CreateProjectRequest {
            name: "project-v1".to_string(),
            local_path: "/path/to/project".to_string(),
            remote_url: None,
            default_branch: None,
            worktree_dir: None,
            workspace_id: workspace_id_1,
            org_id: Some("org-first".to_string()),
        };
        let project1 = store.register(gateway_id.clone(), first).await.unwrap();

        let second = CreateProjectRequest {
            name: "project-v2".to_string(),
            local_path: "/path/to/project".to_string(),
            remote_url: None,
            default_branch: None,
            worktree_dir: None,
            workspace_id: workspace_id_2,
            org_id: Some("org-second".to_string()),
        };
        let project2 = store.register(gateway_id.clone(), second).await.unwrap();

        assert_eq!(project1.id, project2.id);
        assert_eq!(project2.workspace_id, workspace_id_2);
        assert_eq!(project2.org_id, "org-second");
    }

    #[tokio::test]
    async fn test_new_migrates_legacy_projects_without_workspace_id() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("projects.json");
        let default_workspace_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();
        let gateway_id = "host-legacy";

        let legacy_json = format!(
            r#"{{
  "{project_id}": {{
    "id": "{project_id}",
    "name": "legacy-project",
    "local_path": "/path/to/project",
    "remote_url": null,
    "default_branch": "main",
    "gateway_id": "{gateway_id}",
    "worktree_dir": ".worktrees",
    "created_at": "2026-02-08T00:00:00Z",
    "updated_at": "2026-02-08T00:00:00Z"
  }}
}}"#
        );

        tokio::fs::write(&path, legacy_json).await.unwrap();

        let store = ProjectStore::new(path.clone(), default_workspace_id)
            .await
            .unwrap();
        let project = store.get(project_id).await.unwrap();
        assert_eq!(project.workspace_id, default_workspace_id);

        let persisted = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(persisted.contains(&format!("\"workspace_id\": \"{}\"", default_workspace_id)));
    }
}

#[derive(Debug, Deserialize)]
struct StoredProject {
    id: Uuid,
    name: String,
    org_id: Option<String>,
    local_path: String,
    remote_url: Option<String>,
    default_branch: String,
    gateway_id: String,
    workspace_id: Option<Uuid>,
    worktree_dir: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
