//! Project persistent store
//!
//! Provides file-based persistence for projects.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::Error;
use crate::Result;

use super::model::{CreateProjectRequest, Project, ProjectSummary};

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
    pub async fn new(file_path: PathBuf) -> Result<Self> {
        let projects = if file_path.exists() {
            let content = tokio::fs::read_to_string(&file_path).await.map_err(|e| {
                Error::Storage(format!("Failed to read projects file: {}", e))
            })?;
            serde_json::from_str(&content).map_err(|e| {
                Error::Storage(format!("Failed to parse projects file: {}", e))
            })?
        } else {
            HashMap::new()
        };

        Ok(Self {
            projects: Arc::new(RwLock::new(projects)),
            file_path,
        })
    }

    /// Create or update a project from a Gateway registration request
    ///
    /// If a project with the same (gateway_id, local_path) exists, update it.
    /// Otherwise, create a new project.
    pub async fn register(
        &self,
        gateway_id: Uuid,
        request: CreateProjectRequest,
    ) -> Result<Project> {
        let mut projects = self.projects.write().await;

        // Check if project already exists for this gateway + path
        let existing = projects.values().find(|p| {
            p.gateway_id == gateway_id && p.local_path == request.local_path
        });

        let project = if let Some(existing) = existing {
            // Update existing project
            let mut updated = existing.clone();
            updated.name = request.name;
            updated.remote_url = request.remote_url;
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
            let mut project = Project::new(request.name, request.local_path, gateway_id);
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
    pub async fn get_by_gateway_path(&self, gateway_id: Uuid, local_path: &str) -> Option<Project> {
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
    pub async fn list_by_gateway(&self, gateway_id: Uuid) -> Vec<ProjectSummary> {
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
        let content = serde_json::to_string_pretty(&*projects).map_err(|e| {
            Error::Storage(format!("Failed to serialize projects: {}", e))
        })?;

        // Ensure parent directory exists
        if let Some(parent) = self.file_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                Error::Storage(format!("Failed to create directory: {}", e))
            })?;
        }

        tokio::fs::write(&self.file_path, content).await.map_err(|e| {
            Error::Storage(format!("Failed to write projects file: {}", e))
        })?;

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

        let store = ProjectStore::new(path).await.unwrap();
        let projects = store.list().await;

        assert_eq!(projects.len(), 0);
    }

    #[tokio::test]
    async fn test_register_project() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("projects.json");

        let store = ProjectStore::new(path.clone()).await.unwrap();
        let gateway_id = Uuid::new_v4();

        let request = CreateProjectRequest {
            name: "test-project".to_string(),
            local_path: "/path/to/project".to_string(),
            remote_url: Some("git@github.com:user/repo.git".to_string()),
            default_branch: Some("main".to_string()),
            worktree_dir: None,
        };

        let project = store.register(gateway_id, request).await.unwrap();

        assert_eq!(project.name, "test-project");
        assert_eq!(project.gateway_id, gateway_id);

        // Verify persistence
        let store2 = ProjectStore::new(path).await.unwrap();
        let projects = store2.list().await;
        assert_eq!(projects.len(), 1);
    }

    #[tokio::test]
    async fn test_register_project_updates_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("projects.json");

        let store = ProjectStore::new(path).await.unwrap();
        let gateway_id = Uuid::new_v4();

        let request1 = CreateProjectRequest {
            name: "project-v1".to_string(),
            local_path: "/path/to/project".to_string(),
            remote_url: None,
            default_branch: None,
            worktree_dir: None,
        };

        let project1 = store.register(gateway_id, request1).await.unwrap();

        // Register again with same gateway + path
        let request2 = CreateProjectRequest {
            name: "project-v2".to_string(),
            local_path: "/path/to/project".to_string(),
            remote_url: Some("git@github.com:user/repo.git".to_string()),
            default_branch: None,
            worktree_dir: None,
        };

        let project2 = store.register(gateway_id, request2).await.unwrap();

        // Should update, not create new
        assert_eq!(project1.id, project2.id);
        assert_eq!(project2.name, "project-v2");
        assert!(project2.remote_url.is_some());

        let projects = store.list().await;
        assert_eq!(projects.len(), 1);
    }
}
