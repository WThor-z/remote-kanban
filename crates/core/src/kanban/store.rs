//! Kanban board persistent store
//!
//! Provides file-based persistence for kanban board state.
//! Can initialize from TaskStore (tasks.json) for backward compatibility.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::Error;
use crate::task::{FileTaskStore, TaskRepository, TaskStatus};
use crate::Result;

use super::model::{KanbanBoardState, KanbanTask, KanbanTaskStatus};

/// Thread-safe kanban store with file persistence
#[derive(Clone)]
pub struct KanbanStore {
    state: Arc<RwLock<KanbanBoardState>>,
    file_path: PathBuf,
    task_store: Option<Arc<FileTaskStore>>,
}

impl KanbanStore {
    /// Create a new KanbanStore with the given file path
    pub async fn new(file_path: PathBuf) -> Result<Self> {
        let state = if file_path.exists() {
            let content = tokio::fs::read_to_string(&file_path).await.map_err(|e| {
                Error::Storage(format!("Failed to read kanban file: {}", e))
            })?;
            serde_json::from_str(&content).map_err(|e| {
                Error::Storage(format!("Failed to parse kanban file: {}", e))
            })?
        } else {
            KanbanBoardState::new()
        };

        Ok(Self {
            state: Arc::new(RwLock::new(state)),
            file_path,
            task_store: None,
        })
    }

    /// Create a new KanbanStore that syncs with a TaskStore
    /// This loads existing tasks from TaskStore on initialization
    pub async fn with_task_store(file_path: PathBuf, task_store: Arc<FileTaskStore>) -> Result<Self> {
        // First try to load from kanban.json
        let mut state = if file_path.exists() {
            let content = tokio::fs::read_to_string(&file_path).await.map_err(|e| {
                Error::Storage(format!("Failed to read kanban file: {}", e))
            })?;
            serde_json::from_str(&content).unwrap_or_else(|_| KanbanBoardState::new())
        } else {
            KanbanBoardState::new()
        };

        // Sync tasks from TaskStore that aren't in KanbanStore yet
        let tasks = task_store.list().await?;
        for task in tasks {
            let task_id = task.id.to_string();
            if !state.tasks.contains_key(&task_id) {
                // Convert Task to KanbanTask
                let kanban_status = match task.status {
                    TaskStatus::Todo => KanbanTaskStatus::Todo,
                    TaskStatus::InProgress => KanbanTaskStatus::Doing,
                    TaskStatus::InReview => KanbanTaskStatus::Doing,
                    TaskStatus::Done => KanbanTaskStatus::Done,
                };
                
                let mut kanban_task = KanbanTask::new(&task_id, &task.title);
                kanban_task.status = kanban_status;
                if let Some(desc) = &task.description {
                    kanban_task = kanban_task.with_description(desc);
                }
                kanban_task.created_at = task.created_at.timestamp_millis();
                kanban_task.updated_at = Some(task.updated_at.timestamp_millis());
                
                state.add_task(kanban_task);
            }
        }

        let store = Self {
            state: Arc::new(RwLock::new(state)),
            file_path,
            task_store: Some(task_store),
        };
        
        // Persist the synced state
        store.persist().await?;
        
        Ok(store)
    }

    /// Sync new tasks from TaskStore that aren't in KanbanStore yet
    pub async fn sync_from_task_store(&self) -> Result<bool> {
        let Some(task_store) = &self.task_store else {
            return Ok(false);
        };

        let tasks = task_store.list().await?;
        let mut added = false;

        {
            let mut state = self.state.write().await;
            for task in tasks {
                let task_id = task.id.to_string();
                if !state.tasks.contains_key(&task_id) {
                    // Convert Task to KanbanTask
                    let kanban_status = match task.status {
                        TaskStatus::Todo => KanbanTaskStatus::Todo,
                        TaskStatus::InProgress => KanbanTaskStatus::Doing,
                        TaskStatus::InReview => KanbanTaskStatus::Doing,
                        TaskStatus::Done => KanbanTaskStatus::Done,
                    };

                    let mut kanban_task = KanbanTask::new(&task_id, &task.title);
                    kanban_task.status = kanban_status;
                    if let Some(desc) = &task.description {
                        kanban_task = kanban_task.with_description(desc);
                    }
                    kanban_task.created_at = task.created_at.timestamp_millis();
                    kanban_task.updated_at = Some(task.updated_at.timestamp_millis());

                    state.add_task(kanban_task);
                    added = true;
                }
            }
        }

        if added {
            self.persist().await?;
        }

        Ok(added)
    }

    /// Get the current board state
    pub async fn get_state(&self) -> KanbanBoardState {
        self.state.read().await.clone()
    }

    /// Get the current board state after syncing from TaskStore
    pub async fn get_state_synced(&self) -> Result<KanbanBoardState> {
        self.sync_from_task_store().await?;
        Ok(self.state.read().await.clone())
    }

    /// Create a new task
    pub async fn create_task(&self, title: &str, description: Option<&str>) -> Result<KanbanTask> {
        let task_id = format!("task-{}-{}", 
            chrono::Utc::now().timestamp_millis(),
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("0000")
        );
        
        let mut task = KanbanTask::new(&task_id, title);
        if let Some(desc) = description {
            task = task.with_description(desc);
        }

        {
            let mut state = self.state.write().await;
            state.add_task(task.clone());
        }

        self.persist().await?;
        Ok(task)
    }

    /// Move a task to a new status
    pub async fn move_task(
        &self,
        task_id: &str,
        target_status: KanbanTaskStatus,
        target_index: Option<usize>,
    ) -> Result<bool> {
        let result = {
            let mut state = self.state.write().await;
            state.move_task(task_id, target_status, target_index)
        };

        if result {
            self.persist().await?;
        }
        Ok(result)
    }

    /// Delete a task
    pub async fn delete_task(&self, task_id: &str) -> Result<Option<KanbanTask>> {
        let task = {
            let mut state = self.state.write().await;
            state.delete_task(task_id)
        };

        if task.is_some() {
            self.persist().await?;
        }
        Ok(task)
    }

    /// Persist the current state to file
    async fn persist(&self) -> Result<()> {
        let state = self.state.read().await;
        let content = serde_json::to_string_pretty(&*state).map_err(|e| {
            Error::Storage(format!("Failed to serialize kanban state: {}", e))
        })?;

        // Ensure parent directory exists
        if let Some(parent) = self.file_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                Error::Storage(format!("Failed to create directory: {}", e))
            })?;
        }

        tokio::fs::write(&self.file_path, content).await.map_err(|e| {
            Error::Storage(format!("Failed to write kanban file: {}", e))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_kanban_store() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("kanban.json");
        
        let store = KanbanStore::new(path).await.unwrap();
        let state = store.get_state().await;
        
        assert_eq!(state.tasks.len(), 0);
    }

    #[tokio::test]
    async fn test_create_and_persist_task() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("kanban.json");
        
        let store = KanbanStore::new(path.clone()).await.unwrap();
        let task = store.create_task("Test Task", Some("Description")).await.unwrap();
        
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.description, Some("Description".to_string()));
        
        // Verify persistence
        let store2 = KanbanStore::new(path).await.unwrap();
        let state = store2.get_state().await;
        assert_eq!(state.tasks.len(), 1);
    }
}
