//! Application state

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use socketioxide::SocketIo;

use agent_runner::{ExecutorConfig, TaskExecutor};
use git_worktree::WorktreeConfig;
use vk_core::kanban::KanbanStore;
use vk_core::task::FileTaskStore;

use crate::gateway::GatewayManager;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    pub task_store: Arc<FileTaskStore>,
    pub kanban_store: Arc<KanbanStore>,
    pub executor: Arc<TaskExecutor>,
    #[allow(dead_code)]
    pub repo_path: PathBuf,
    pub socket_io: Arc<RwLock<Option<SocketIo>>>,
    pub gateway_manager: Arc<GatewayManager>,
}

impl AppState {
    /// Create a new AppState with the given data directory and gateway manager
    /// (creates its own TaskStore)
    #[allow(dead_code)]
    pub async fn new(data_dir: PathBuf, gateway_manager: Arc<GatewayManager>) -> vk_core::Result<Self> {
        let tasks_path = data_dir.join("tasks.json");
        let task_store = Arc::new(FileTaskStore::new(tasks_path).await?);
        
        // Create kanban store
        let kanban_path = data_dir.join("kanban.json");
        let kanban_store = Arc::new(KanbanStore::with_task_store(kanban_path, Arc::clone(&task_store)).await?);
        
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
                executor: Arc::new(executor),
                repo_path,
                socket_io: Arc::new(RwLock::new(None)),
                gateway_manager,
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
}
