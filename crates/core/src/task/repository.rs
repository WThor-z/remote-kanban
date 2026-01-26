//! Task repository trait
//!
//! Defines the interface for task storage operations.

use async_trait::async_trait;
use uuid::Uuid;

use super::model::{Task, TaskStatus};
use crate::Result;

/// Repository interface for task CRUD operations
#[async_trait]
pub trait TaskRepository: Send + Sync {
    /// Create a new task
    async fn create(&self, task: Task) -> Result<Task>;

    /// Get a task by ID
    async fn get(&self, id: Uuid) -> Result<Option<Task>>;

    /// Get all tasks
    async fn list(&self) -> Result<Vec<Task>>;

    /// Update an existing task
    async fn update(&self, task: Task) -> Result<Task>;

    /// Delete a task by ID
    async fn delete(&self, id: Uuid) -> Result<bool>;

    /// Find tasks by status
    async fn find_by_status(&self, status: TaskStatus) -> Result<Vec<Task>>;
}
