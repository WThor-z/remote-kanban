//! Application state

use std::path::PathBuf;
use std::sync::Arc;

use vk_core::task::FileTaskStore;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    pub task_store: Arc<FileTaskStore>,
}

impl AppState {
    /// Create a new AppState with the given data directory
    pub async fn new(data_dir: PathBuf) -> vk_core::Result<Self> {
        let tasks_path = data_dir.join("tasks.json");
        let task_store = Arc::new(FileTaskStore::new(tasks_path).await?);

        Ok(Self {
            inner: Arc::new(AppStateInner { task_store }),
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
}
