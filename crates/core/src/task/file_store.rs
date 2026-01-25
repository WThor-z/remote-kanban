//! File-based task storage implementation
//!
//! Stores tasks as JSON in a file on disk.

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::model::{Task, TaskStatus};
use super::repository::TaskRepository;
use crate::{Error, Result};

/// File-based task store using JSON
pub struct FileTaskStore {
    /// Path to the JSON file
    path: PathBuf,
    /// In-memory cache of tasks
    cache: RwLock<HashMap<Uuid, Task>>,
}

impl FileTaskStore {
    /// Create a new FileTaskStore
    ///
    /// If the file doesn't exist, it will be created on first write.
    pub async fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let cache = if path.exists() {
            let content = tokio::fs::read_to_string(&path).await?;
            let tasks: Vec<Task> = serde_json::from_str(&content)?;
            tasks.into_iter().map(|t| (t.id, t)).collect()
        } else {
            HashMap::new()
        };

        Ok(Self {
            path,
            cache: RwLock::new(cache),
        })
    }

    /// Persist the cache to disk
    async fn persist(&self) -> Result<()> {
        let cache = self.cache.read().await;
        let tasks: Vec<&Task> = cache.values().collect();
        let content = serde_json::to_string_pretty(&tasks)?;

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&self.path, content).await?;
        Ok(())
    }
}

#[async_trait]
impl TaskRepository for FileTaskStore {
    async fn create(&self, task: Task) -> Result<Task> {
        {
            let mut cache = self.cache.write().await;
            if cache.contains_key(&task.id) {
                return Err(Error::InvalidInput(format!(
                    "Task with ID {} already exists",
                    task.id
                )));
            }
            cache.insert(task.id, task.clone());
        }
        self.persist().await?;
        Ok(task)
    }

    async fn get(&self, id: Uuid) -> Result<Option<Task>> {
        let cache = self.cache.read().await;
        Ok(cache.get(&id).cloned())
    }

    async fn list(&self) -> Result<Vec<Task>> {
        let cache = self.cache.read().await;
        let mut tasks: Vec<Task> = cache.values().cloned().collect();
        // Sort by created_at descending (newest first)
        tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(tasks)
    }

    async fn update(&self, mut task: Task) -> Result<Task> {
        task.updated_at = Utc::now();
        {
            let mut cache = self.cache.write().await;
            if !cache.contains_key(&task.id) {
                return Err(Error::TaskNotFound(task.id.to_string()));
            }
            cache.insert(task.id, task.clone());
        }
        self.persist().await?;
        Ok(task)
    }

    async fn delete(&self, id: Uuid) -> Result<bool> {
        let removed = {
            let mut cache = self.cache.write().await;
            cache.remove(&id).is_some()
        };
        if removed {
            self.persist().await?;
        }
        Ok(removed)
    }

    async fn find_by_status(&self, status: TaskStatus) -> Result<Vec<Task>> {
        let cache = self.cache.read().await;
        let mut tasks: Vec<Task> = cache
            .values()
            .filter(|t| t.status == status)
            .cloned()
            .collect();
        tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{TaskPriority, TaskStatus};
    use tempfile::TempDir;

    async fn create_test_store() -> (FileTaskStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("tasks.json");
        let store = FileTaskStore::new(&path).await.unwrap();
        (store, temp_dir)
    }

    #[tokio::test]
    async fn test_create_task() {
        let (store, _temp) = create_test_store().await;

        let task = Task::new("Test task").with_description("A test description");
        let created = store.create(task.clone()).await.unwrap();

        assert_eq!(created.id, task.id);
        assert_eq!(created.title, "Test task");
        assert_eq!(created.description, Some("A test description".to_string()));
    }

    #[tokio::test]
    async fn test_get_task() {
        let (store, _temp) = create_test_store().await;

        let task = Task::new("Test task");
        let id = task.id;
        store.create(task).await.unwrap();

        let retrieved = store.get(id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, id);

        // Test non-existent task
        let non_existent = store.get(Uuid::new_v4()).await.unwrap();
        assert!(non_existent.is_none());
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let (store, _temp) = create_test_store().await;

        // Create multiple tasks
        store.create(Task::new("Task 1")).await.unwrap();
        store.create(Task::new("Task 2")).await.unwrap();
        store.create(Task::new("Task 3")).await.unwrap();

        let tasks = store.list().await.unwrap();
        assert_eq!(tasks.len(), 3);
    }

    #[tokio::test]
    async fn test_update_task() {
        let (store, _temp) = create_test_store().await;

        let task = Task::new("Original title");
        let id = task.id;
        store.create(task).await.unwrap();

        let mut updated_task = store.get(id).await.unwrap().unwrap();
        updated_task.title = "Updated title".to_string();
        updated_task.status = TaskStatus::InProgress;

        let result = store.update(updated_task).await.unwrap();
        assert_eq!(result.title, "Updated title");
        assert_eq!(result.status, TaskStatus::InProgress);

        // Verify persistence
        let retrieved = store.get(id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated title");
    }

    #[tokio::test]
    async fn test_update_nonexistent_task() {
        let (store, _temp) = create_test_store().await;

        let task = Task::new("Test task");
        let result = store.update(task).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::TaskNotFound(_) => {}
            e => panic!("Expected TaskNotFound error, got: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_delete_task() {
        let (store, _temp) = create_test_store().await;

        let task = Task::new("Task to delete");
        let id = task.id;
        store.create(task).await.unwrap();

        // Verify task exists
        assert!(store.get(id).await.unwrap().is_some());

        // Delete task
        let deleted = store.delete(id).await.unwrap();
        assert!(deleted);

        // Verify task is gone
        assert!(store.get(id).await.unwrap().is_none());

        // Delete again should return false
        let deleted_again = store.delete(id).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_find_by_status() {
        let (store, _temp) = create_test_store().await;

        // Create tasks with different statuses
        store.create(Task::new("Todo 1")).await.unwrap();
        store.create(Task::new("Todo 2")).await.unwrap();

        let mut in_progress = Task::new("In Progress 1");
        in_progress.status = TaskStatus::InProgress;
        store.create(in_progress).await.unwrap();

        let mut done = Task::new("Done 1");
        done.status = TaskStatus::Done;
        store.create(done).await.unwrap();

        // Find by status
        let todos = store.find_by_status(TaskStatus::Todo).await.unwrap();
        assert_eq!(todos.len(), 2);

        let in_progress_tasks = store.find_by_status(TaskStatus::InProgress).await.unwrap();
        assert_eq!(in_progress_tasks.len(), 1);

        let done_tasks = store.find_by_status(TaskStatus::Done).await.unwrap();
        assert_eq!(done_tasks.len(), 1);

        let in_review_tasks = store.find_by_status(TaskStatus::InReview).await.unwrap();
        assert_eq!(in_review_tasks.len(), 0);
    }

    #[tokio::test]
    async fn test_persistence_across_instances() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("tasks.json");

        let task_id;

        // Create store and add task
        {
            let store = FileTaskStore::new(&path).await.unwrap();
            let task = Task::new("Persistent task")
                .with_description("Should survive reload")
                .with_priority(TaskPriority::High);
            task_id = task.id;
            store.create(task).await.unwrap();
        }

        // Create new store instance and verify data persisted
        {
            let store = FileTaskStore::new(&path).await.unwrap();
            let task = store.get(task_id).await.unwrap();
            assert!(task.is_some());
            let task = task.unwrap();
            assert_eq!(task.title, "Persistent task");
            assert_eq!(task.description, Some("Should survive reload".to_string()));
            assert_eq!(task.priority, TaskPriority::High);
        }
    }

    #[tokio::test]
    async fn test_duplicate_task_error() {
        let (store, _temp) = create_test_store().await;

        let task = Task::new("Test task");
        store.create(task.clone()).await.unwrap();

        // Try to create same task again
        let result = store.create(task).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InvalidInput(msg) => {
                assert!(msg.contains("already exists"));
            }
            e => panic!("Expected InvalidInput error, got: {:?}", e),
        }
    }
}
