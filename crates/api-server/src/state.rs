//! Application state

use socketioxide::SocketIo;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use agent_runner::{ExecutorConfig, TaskExecutor};
use git_worktree::WorktreeConfig;
use vk_core::kanban::KanbanStore;
use vk_core::project::ProjectStore;
use vk_core::task::FileTaskStore;
use vk_core::workspace::{CreateWorkspaceRequest, WorkspaceStore, WorkspaceSummary};

use crate::audit::AuditStore;
use crate::auth::AuthStore;
use crate::gateway::GatewayManager;
use crate::memory::MemoryStore;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    pub task_store: Arc<FileTaskStore>,
    pub kanban_store: Arc<KanbanStore>,
    pub project_store: Arc<ProjectStore>,
    pub workspace_store: Arc<WorkspaceStore>,
    pub executor: Arc<TaskExecutor>,
    #[allow(dead_code)]
    pub repo_path: PathBuf,
    pub socket_io: Arc<RwLock<Option<SocketIo>>>,
    pub gateway_manager: Arc<GatewayManager>,
    pub memory_store: Arc<MemoryStore>,
    pub audit_store: Arc<AuditStore>,
    pub auth_store: Arc<AuthStore>,
}

impl AppState {
    /// Create a new AppState with the given data directory and gateway manager
    /// (creates its own TaskStore)
    #[allow(dead_code)]
    pub async fn new(
        data_dir: PathBuf,
        gateway_manager: Arc<GatewayManager>,
    ) -> vk_core::Result<Self> {
        let tasks_path = data_dir.join("tasks.json");
        let task_store = Arc::new(FileTaskStore::new(tasks_path).await?);

        // Create kanban store
        let kanban_path = data_dir.join("kanban.json");
        let kanban_store =
            Arc::new(KanbanStore::with_task_store(kanban_path, Arc::clone(&task_store)).await?);

        Self::with_stores(data_dir, task_store, kanban_store, gateway_manager).await
    }

    /// Create a new AppState with pre-created stores
    pub async fn with_stores(
        data_dir: PathBuf,
        task_store: Arc<FileTaskStore>,
        kanban_store: Arc<KanbanStore>,
        gateway_manager: Arc<GatewayManager>,
    ) -> vk_core::Result<Self> {
        // Get repository path (current directory or from env)
        let repo_path = std::env::var("VK_REPO_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let workspace_path = data_dir.join("workspaces.json");
        let workspace_store = Arc::new(WorkspaceStore::new(workspace_path).await?);
        let workspace_summaries = workspace_store.list().await;
        let default_workspace_id = if let Some(workspace) = workspace_summaries
            .iter()
            .find(|workspace| workspace.archived_at.is_none())
        {
            workspace.id
        } else {
            let slug = next_default_workspace_slug(&workspace_summaries);
            workspace_store
                .create(CreateWorkspaceRequest {
                    name: "Default Workspace".to_string(),
                    slug: Some(slug),
                    org_id: None,
                    host_id: default_workspace_host_id(),
                    root_path: repo_path.to_string_lossy().to_string(),
                    default_project_id: None,
                })
                .await?
                .id
        };

        let project_path = data_dir.join("projects.json");
        let project_store = Arc::new(ProjectStore::new(project_path, default_workspace_id).await?);
        let memory_store = Arc::new(MemoryStore::new(data_dir.join("memory")).await.map_err(
            |e| vk_core::Error::Storage(format!("Failed to initialize memory store: {}", e)),
        )?);
        let audit_store = Arc::new(AuditStore::new(data_dir.join("audit")).await.map_err(|e| {
            vk_core::Error::Storage(format!("Failed to initialize audit store: {}", e))
        })?);
        let auth_store = Arc::new(AuthStore::new(data_dir.join("auth")).await.map_err(|e| {
            vk_core::Error::Storage(format!("Failed to initialize auth store: {}", e))
        })?);

        // Create executor config
        let executor_config = ExecutorConfig {
            data_dir: data_dir.clone(),
            repo_path: repo_path.clone(),
            worktree_config: WorktreeConfig {
                worktree_dir: data_dir.join("worktrees"),
                branch_prefix: "task/".to_string(),
            },
            auto_cleanup: false,
            delete_branches: false,
        };

        // Create task executor
        let executor = TaskExecutor::new(executor_config)
            .await
            .map_err(|e| vk_core::Error::Storage(format!("Failed to create executor: {}", e)))?;

        Ok(Self {
            inner: Arc::new(AppStateInner {
                task_store,
                kanban_store,
                project_store,
                workspace_store,
                executor: Arc::new(executor),
                repo_path,
                socket_io: Arc::new(RwLock::new(None)),
                gateway_manager,
                memory_store,
                audit_store,
                auth_store,
            }),
        })
    }

    /// Set Socket.IO instance
    pub async fn set_socket_io(&self, io: SocketIo) {
        let mut w = self.inner.socket_io.write().await;
        *w = Some(io);
    }

    /// Get Socket.IO instance
    pub async fn get_socket_io(&self) -> Option<SocketIo> {
        self.inner.socket_io.read().await.clone()
    }

    /// Get reference to the task store
    pub fn task_store(&self) -> &FileTaskStore {
        &self.inner.task_store
    }

    /// Get shared Arc to the task store (for KanbanStore integration)
    #[allow(dead_code)]
    pub fn task_store_arc(&self) -> Arc<FileTaskStore> {
        Arc::clone(&self.inner.task_store)
    }

    /// Get reference to the kanban store
    pub fn kanban_store(&self) -> &KanbanStore {
        &self.inner.kanban_store
    }

    /// Get shared Arc to the kanban store
    #[allow(dead_code)]
    pub fn kanban_store_arc(&self) -> Arc<KanbanStore> {
        Arc::clone(&self.inner.kanban_store)
    }

    /// Get reference to the project store
    pub fn project_store(&self) -> &ProjectStore {
        &self.inner.project_store
    }

    /// Get shared Arc to the project store
    #[allow(dead_code)]
    pub fn project_store_arc(&self) -> Arc<ProjectStore> {
        Arc::clone(&self.inner.project_store)
    }

    /// Get reference to the workspace store
    pub fn workspace_store(&self) -> &WorkspaceStore {
        &self.inner.workspace_store
    }

    /// Get shared Arc to the workspace store
    #[allow(dead_code)]
    pub fn workspace_store_arc(&self) -> Arc<WorkspaceStore> {
        Arc::clone(&self.inner.workspace_store)
    }

    /// Get reference to the task executor
    pub fn executor(&self) -> &TaskExecutor {
        &self.inner.executor
    }

    /// Get the repository path
    #[allow(dead_code)]
    pub fn repo_path(&self) -> &PathBuf {
        &self.inner.repo_path
    }

    /// Get reference to the gateway manager
    pub fn gateway_manager(&self) -> &GatewayManager {
        &self.inner.gateway_manager
    }

    /// Get shared Arc to the gateway manager
    pub fn gateway_manager_arc(&self) -> Arc<GatewayManager> {
        Arc::clone(&self.inner.gateway_manager)
    }

    pub fn memory_store(&self) -> &MemoryStore {
        &self.inner.memory_store
    }

    pub fn memory_store_arc(&self) -> Arc<MemoryStore> {
        Arc::clone(&self.inner.memory_store)
    }

    pub fn audit_store(&self) -> &AuditStore {
        &self.inner.audit_store
    }

    #[allow(dead_code)]
    pub fn audit_store_arc(&self) -> Arc<AuditStore> {
        Arc::clone(&self.inner.audit_store)
    }

    pub fn auth_store(&self) -> &AuthStore {
        &self.inner.auth_store
    }

    #[allow(dead_code)]
    pub fn auth_store_arc(&self) -> Arc<AuthStore> {
        Arc::clone(&self.inner.auth_store)
    }
}

fn next_default_workspace_slug(workspaces: &[WorkspaceSummary]) -> String {
    let existing: HashSet<&str> = workspaces
        .iter()
        .map(|workspace| workspace.slug.as_str())
        .collect();
    if !existing.contains("default") {
        return "default".to_string();
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("default-{}", suffix);
        if !existing.contains(candidate.as_str()) {
            return candidate;
        }
        suffix += 1;
    }
}

fn default_workspace_host_id() -> String {
    if let Ok(host_id) = std::env::var("GATEWAY_HOST_ID") {
        let trimmed = host_id.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let raw_hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "gateway-host".to_string());
    let hostname = raw_hostname.trim().to_lowercase();

    let mut sanitized = String::with_capacity(hostname.len());
    let mut last_was_dash = false;
    for ch in hostname.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            sanitized.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            sanitized.push('-');
            last_was_dash = true;
        }
    }

    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "host-gateway-host".to_string()
    } else {
        format!("host-{}", trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;
    use uuid::Uuid;
    use vk_core::{
        kanban::KanbanStore,
        project::CreateProjectRequest,
        task::FileTaskStore,
        workspace::{CreateWorkspaceRequest, WorkspaceStore},
    };

    #[tokio::test]
    async fn app_state_exposes_workspace_store() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let tasks_path = data_dir.join("tasks.json");
        let task_store = Arc::new(FileTaskStore::new(tasks_path).await.unwrap());
        let kanban_path = data_dir.join("kanban.json");
        let kanban_store = Arc::new(
            KanbanStore::with_task_store(kanban_path, Arc::clone(&task_store))
                .await
                .unwrap(),
        );
        let gateway_manager = Arc::new(GatewayManager::with_stores(
            Arc::clone(&task_store),
            Arc::clone(&kanban_store),
        ));

        let state = AppState::with_stores(
            data_dir,
            Arc::clone(&task_store),
            Arc::clone(&kanban_store),
            Arc::clone(&gateway_manager),
        )
        .await
        .unwrap();

        let workspace = state
            .workspace_store()
            .create(CreateWorkspaceRequest {
                name: "Workspace One".to_string(),
                slug: None,
                host_id: "host-1".to_string(),
                root_path: "/tmp/workspace-one".to_string(),
                default_project_id: None,
                org_id: None,
            })
            .await
            .unwrap();

        let loaded = state.workspace_store().get(workspace.id).await;
        assert!(loaded.is_some());
    }

    #[tokio::test]
    async fn app_state_bootstraps_default_workspace_for_project_store() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let tasks_path = data_dir.join("tasks.json");
        let task_store = Arc::new(FileTaskStore::new(tasks_path).await.unwrap());
        let kanban_path = data_dir.join("kanban.json");
        let kanban_store = Arc::new(
            KanbanStore::with_task_store(kanban_path, Arc::clone(&task_store))
                .await
                .unwrap(),
        );
        let gateway_manager = Arc::new(GatewayManager::with_stores(
            Arc::clone(&task_store),
            Arc::clone(&kanban_store),
        ));

        let state = AppState::with_stores(
            data_dir,
            Arc::clone(&task_store),
            Arc::clone(&kanban_store),
            Arc::clone(&gateway_manager),
        )
        .await
        .unwrap();

        let workspaces = state.workspace_store().list().await;
        assert_eq!(workspaces.len(), 1);
        let default_workspace_id = workspaces[0].id;
        assert_eq!(
            workspaces[0].root_path,
            state.repo_path().to_string_lossy().to_string()
        );

        let project = state
            .project_store()
            .register(
                "host-1".to_string(),
                CreateProjectRequest {
                    name: "Project One".to_string(),
                    local_path: "/tmp/project-one".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id: default_workspace_id,
                    org_id: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(project.workspace_id, default_workspace_id);
    }

    #[tokio::test]
    async fn app_state_bootstraps_active_default_workspace_when_existing_workspaces_are_archived() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let tasks_path = data_dir.join("tasks.json");
        let task_store = Arc::new(FileTaskStore::new(tasks_path).await.unwrap());
        let kanban_path = data_dir.join("kanban.json");
        let kanban_store = Arc::new(
            KanbanStore::with_task_store(kanban_path, Arc::clone(&task_store))
                .await
                .unwrap(),
        );
        let gateway_manager = Arc::new(GatewayManager::with_stores(
            Arc::clone(&task_store),
            Arc::clone(&kanban_store),
        ));

        let workspace_store = WorkspaceStore::new(data_dir.join("workspaces.json"))
            .await
            .unwrap();
        let archived_workspace = workspace_store
            .create(CreateWorkspaceRequest {
                name: "Archived Workspace".to_string(),
                slug: Some("archived".to_string()),
                host_id: "host-archived".to_string(),
                root_path: "/tmp/archived-workspace".to_string(),
                default_project_id: None,
                org_id: None,
            })
            .await
            .unwrap()
            .archive();
        let archived_workspace = workspace_store.update(archived_workspace).await.unwrap();

        let project_id = Uuid::new_v4();
        let gateway_id = "host-legacy";
        let legacy_project_json = format!(
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
        tokio::fs::write(data_dir.join("projects.json"), legacy_project_json)
            .await
            .unwrap();

        let state = AppState::with_stores(
            data_dir,
            Arc::clone(&task_store),
            Arc::clone(&kanban_store),
            Arc::clone(&gateway_manager),
        )
        .await
        .unwrap();

        let workspaces = state.workspace_store().list().await;
        assert_eq!(workspaces.len(), 2);

        let active_workspace = workspaces
            .iter()
            .find(|workspace| workspace.archived_at.is_none())
            .expect("expected an active workspace")
            .clone();
        assert_eq!(active_workspace.slug, "default");
        assert_eq!(
            active_workspace.root_path,
            state.repo_path().to_string_lossy().to_string()
        );
        assert_ne!(active_workspace.id, archived_workspace.id);

        let migrated_project = state.project_store().get(project_id).await.unwrap();
        assert_eq!(migrated_project.workspace_id, active_workspace.id);
        assert_ne!(migrated_project.workspace_id, archived_workspace.id);
    }

    #[tokio::test]
    async fn app_state_bootstraps_unique_default_slug_when_archived_default_exists() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let tasks_path = data_dir.join("tasks.json");
        let task_store = Arc::new(FileTaskStore::new(tasks_path).await.unwrap());
        let kanban_path = data_dir.join("kanban.json");
        let kanban_store = Arc::new(
            KanbanStore::with_task_store(kanban_path, Arc::clone(&task_store))
                .await
                .unwrap(),
        );
        let gateway_manager = Arc::new(GatewayManager::with_stores(
            Arc::clone(&task_store),
            Arc::clone(&kanban_store),
        ));

        let workspace_store = WorkspaceStore::new(data_dir.join("workspaces.json"))
            .await
            .unwrap();
        let archived_default = workspace_store
            .create(CreateWorkspaceRequest {
                name: "Archived Default".to_string(),
                slug: Some("default".to_string()),
                host_id: "host-archived".to_string(),
                root_path: "/tmp/archived-default".to_string(),
                default_project_id: None,
                org_id: None,
            })
            .await
            .unwrap()
            .archive();
        workspace_store.update(archived_default).await.unwrap();

        let state = AppState::with_stores(
            data_dir,
            Arc::clone(&task_store),
            Arc::clone(&kanban_store),
            Arc::clone(&gateway_manager),
        )
        .await
        .unwrap();

        let workspaces = state.workspace_store().list().await;
        assert_eq!(workspaces.len(), 2);

        let active_workspace = workspaces
            .iter()
            .find(|workspace| workspace.archived_at.is_none())
            .expect("expected an active workspace");
        assert_eq!(active_workspace.slug, "default-2");
    }
}
