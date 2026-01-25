//! Kanban board model definitions
//!
//! These types are designed to be compatible with the frontend's expected format.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Kanban task status - matches frontend's three-column layout
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KanbanTaskStatus {
    Todo,
    Doing,
    Done,
}

impl Default for KanbanTaskStatus {
    fn default() -> Self {
        Self::Todo
    }
}

/// A task in the kanban board (frontend-compatible format)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KanbanTask {
    pub id: String,
    pub title: String,
    pub status: KanbanTaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
    /// Associated agent session ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

impl KanbanTask {
    /// Create a new kanban task
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: id.into(),
            title: title.into(),
            status: KanbanTaskStatus::default(),
            description: None,
            created_at: now,
            updated_at: None,
            session_id: None,
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// A column in the kanban board
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KanbanColumn {
    pub id: KanbanTaskStatus,
    pub title: String,
    pub task_ids: Vec<String>,
}

/// The complete kanban board state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KanbanBoardState {
    pub tasks: HashMap<String, KanbanTask>,
    pub columns: HashMap<KanbanTaskStatus, KanbanColumn>,
    pub column_order: Vec<KanbanTaskStatus>,
}

impl Default for KanbanBoardState {
    fn default() -> Self {
        Self::new()
    }
}

impl KanbanBoardState {
    /// Create a new empty board state
    pub fn new() -> Self {
        let mut columns = HashMap::new();
        columns.insert(
            KanbanTaskStatus::Todo,
            KanbanColumn {
                id: KanbanTaskStatus::Todo,
                title: "To Do".to_string(),
                task_ids: Vec::new(),
            },
        );
        columns.insert(
            KanbanTaskStatus::Doing,
            KanbanColumn {
                id: KanbanTaskStatus::Doing,
                title: "Doing".to_string(),
                task_ids: Vec::new(),
            },
        );
        columns.insert(
            KanbanTaskStatus::Done,
            KanbanColumn {
                id: KanbanTaskStatus::Done,
                title: "Done".to_string(),
                task_ids: Vec::new(),
            },
        );

        Self {
            tasks: HashMap::new(),
            columns,
            column_order: vec![
                KanbanTaskStatus::Todo,
                KanbanTaskStatus::Doing,
                KanbanTaskStatus::Done,
            ],
        }
    }

    /// Add a task to the board
    pub fn add_task(&mut self, task: KanbanTask) {
        let status = task.status;
        let id = task.id.clone();
        self.tasks.insert(id.clone(), task);
        if let Some(column) = self.columns.get_mut(&status) {
            column.task_ids.push(id);
        }
    }

    /// Move a task to a new status
    pub fn move_task(
        &mut self,
        task_id: &str,
        target_status: KanbanTaskStatus,
        target_index: Option<usize>,
    ) -> bool {
        // Get current task
        let Some(task) = self.tasks.get_mut(task_id) else {
            return false;
        };

        let old_status = task.status;
        task.status = target_status;
        task.updated_at = Some(chrono::Utc::now().timestamp_millis());

        // Remove from old column
        if let Some(old_column) = self.columns.get_mut(&old_status) {
            old_column.task_ids.retain(|id| id != task_id);
        }

        // Add to new column
        if let Some(new_column) = self.columns.get_mut(&target_status) {
            let index = target_index.unwrap_or(new_column.task_ids.len());
            let index = index.min(new_column.task_ids.len());
            new_column.task_ids.insert(index, task_id.to_string());
        }

        true
    }

    /// Delete a task from the board
    pub fn delete_task(&mut self, task_id: &str) -> Option<KanbanTask> {
        let task = self.tasks.remove(task_id)?;

        // Remove from column
        if let Some(column) = self.columns.get_mut(&task.status) {
            column.task_ids.retain(|id| id != task_id);
        }

        Some(task)
    }

    /// Get a task by ID
    pub fn get_task(&self, task_id: &str) -> Option<&KanbanTask> {
        self.tasks.get(task_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_board_state() {
        let state = KanbanBoardState::new();
        assert_eq!(state.tasks.len(), 0);
        assert_eq!(state.columns.len(), 3);
        assert_eq!(state.column_order.len(), 3);
    }

    #[test]
    fn test_add_task() {
        let mut state = KanbanBoardState::new();
        let task = KanbanTask::new("task-1", "Test Task");
        state.add_task(task);

        assert_eq!(state.tasks.len(), 1);
        assert_eq!(
            state
                .columns
                .get(&KanbanTaskStatus::Todo)
                .unwrap()
                .task_ids
                .len(),
            1
        );
    }

    #[test]
    fn test_move_task() {
        let mut state = KanbanBoardState::new();
        let task = KanbanTask::new("task-1", "Test Task");
        state.add_task(task);

        let result = state.move_task("task-1", KanbanTaskStatus::Doing, None);
        assert!(result);

        let task = state.get_task("task-1").unwrap();
        assert_eq!(task.status, KanbanTaskStatus::Doing);
        assert!(state
            .columns
            .get(&KanbanTaskStatus::Todo)
            .unwrap()
            .task_ids
            .is_empty());
        assert_eq!(
            state
                .columns
                .get(&KanbanTaskStatus::Doing)
                .unwrap()
                .task_ids
                .len(),
            1
        );
    }

    #[test]
    fn test_delete_task() {
        let mut state = KanbanBoardState::new();
        let task = KanbanTask::new("task-1", "Test Task");
        state.add_task(task);

        let deleted = state.delete_task("task-1");
        assert!(deleted.is_some());
        assert_eq!(state.tasks.len(), 0);
        assert!(state
            .columns
            .get(&KanbanTaskStatus::Todo)
            .unwrap()
            .task_ids
            .is_empty());
    }
}
