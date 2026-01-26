//! Application state

use std::path::PathBuf;
use std::sync::Arc;

use agent_runner::{ExecutorConfig, TaskExecutor};
use git_worktree::WorktreeConfig;
use vk_core::task::FileTaskStore;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    pub task_store: Arc<FileTaskStore>,
    pub executor: Arc<TaskExecutor>,
    #[allow(dead_code)]
    pub repo_path: PathBuf,
}

impl AppState {
    /// Create a new AppState with the given data directory
    pub async fn new(data_dir: PathBuf) -> vk_core::Result<Self> {
        let tasks_path = data_dir.join("tasks.json");
        let task_store = Arc::new(FileTaskStore::new(tasks_path).await?);

        // Get repository path (current directory or from env)
        let repo_path = std::env::var("VK_REPO_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Create executor config
        let executor_config = ExecutorConfig {
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
                executor: Arc::new(executor),
                repo_path,
            }),
        })
    }

    /// Get reference to the task store
    pub fn task_store(&self) -> &FileTaskStore {
        &self.inner.task_store
    }

    /// Get shared Arc to the task store (for KanbanStore integration)
    pub fn task_store_arc(&self) -> Arc<FileTaskStore> {
        Arc::clone(&self.inner.task_store)
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
}
