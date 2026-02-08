//! Run persistence - Store runs and events to disk
//!
//! Directory structure:
//! ```text
//! .vk-data/
//!   runs/
//!     {task_id}/
//!       {run_id}/
//!         run.json       # Run metadata
//!         events.jsonl   # Event log (newline-delimited JSON)
//!         messages.jsonl # Chat messages (newline-delimited JSON)
//! ```

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::error::{ExecutorError, Result};
use crate::event::{AgentEvent, ExecutionEvent, ExecutionEventType};
use crate::run::{ChatMessage, Run, RunSummary};

/// Run store for persisting runs and events
#[derive(Debug, Clone)]
pub struct RunStore {
    /// Base directory for run storage
    base_dir: PathBuf,
}

impl RunStore {
    /// Create a new run store
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        Self {
            base_dir: data_dir.as_ref().join("runs"),
        }
    }

    /// Get the directory path for a task's runs
    fn task_dir(&self, task_id: Uuid) -> PathBuf {
        self.base_dir.join(task_id.to_string())
    }

    /// Get the directory path for a specific run
    fn run_dir(&self, task_id: Uuid, run_id: Uuid) -> PathBuf {
        self.task_dir(task_id).join(run_id.to_string())
    }

    /// Get the path to a run's metadata file
    fn run_metadata_path(&self, task_id: Uuid, run_id: Uuid) -> PathBuf {
        self.run_dir(task_id, run_id).join("run.json")
    }

    /// Get the path to a run's events log file
    fn events_path(&self, task_id: Uuid, run_id: Uuid) -> PathBuf {
        self.run_dir(task_id, run_id).join("events.jsonl")
    }

    /// Get the path to a run's messages log file
    fn messages_path(&self, task_id: Uuid, run_id: Uuid) -> PathBuf {
        self.run_dir(task_id, run_id).join("messages.jsonl")
    }

    /// Ensure the run directory exists
    fn ensure_run_dir(&self, task_id: Uuid, run_id: Uuid) -> Result<PathBuf> {
        let dir = self.run_dir(task_id, run_id);
        if !dir.exists() {
            fs::create_dir_all(&dir).map_err(ExecutorError::from)?;
        }
        Ok(dir)
    }

    /// Save a run's metadata
    pub fn save_run(&self, run: &Run) -> Result<()> {
        self.ensure_run_dir(run.task_id, run.id)?;
        let path = self.run_metadata_path(run.task_id, run.id);

        let file = File::create(&path).map_err(ExecutorError::from)?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, run).map_err(|e| {
            ExecutorError::execution_failed(format!("Failed to serialize run: {}", e))
        })?;

        debug!("Saved run metadata: {}", path.display());
        Ok(())
    }

    /// Load a run's metadata
    pub fn load_run(&self, task_id: Uuid, run_id: Uuid) -> Result<Run> {
        let path = self.run_metadata_path(task_id, run_id);

        let file = File::open(&path).map_err(ExecutorError::from)?;

        let reader = BufReader::new(file);
        let run: Run = serde_json::from_reader(reader).map_err(|e| {
            ExecutorError::execution_failed(format!("Failed to deserialize run: {}", e))
        })?;

        Ok(run)
    }

    /// List all runs for a task
    pub fn list_runs(&self, task_id: Uuid) -> Result<Vec<RunSummary>> {
        let task_dir = self.task_dir(task_id);

        if !task_dir.exists() {
            return Ok(Vec::new());
        }

        let mut runs = Vec::new();

        let entries = fs::read_dir(&task_dir).map_err(ExecutorError::from)?;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("Failed to read directory entry: {}", e);
                    continue;
                }
            };

            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            // Try to parse the directory name as a UUID (run_id)
            let run_id = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => match Uuid::parse_str(name) {
                    Ok(id) => id,
                    Err(_) => continue,
                },
                None => continue,
            };

            // Try to load the run metadata
            match self.load_run(task_id, run_id) {
                Ok(run) => runs.push(RunSummary::from(&run)),
                Err(e) => {
                    warn!("Failed to load run {}: {}", run_id, e);
                    continue;
                }
            }
        }

        // Sort by created_at descending (newest first)
        runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(runs)
    }

    /// Append an event to a run's event log
    pub fn append_event(&self, task_id: Uuid, run_id: Uuid, event: &ExecutionEvent) -> Result<()> {
        self.ensure_run_dir(task_id, run_id)?;
        let path = self.events_path(task_id, run_id);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(ExecutorError::from)?;

        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string(event).map_err(|e| {
            ExecutorError::execution_failed(format!("Failed to serialize event: {}", e))
        })?;

        writeln!(writer, "{}", json).map_err(ExecutorError::from)?;

        writer.flush().map_err(ExecutorError::from)?;

        Ok(())
    }

    /// Load all events for a run
    pub fn load_events(&self, task_id: Uuid, run_id: Uuid) -> Result<Vec<ExecutionEvent>> {
        let path = self.events_path(task_id, run_id);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&path).map_err(ExecutorError::from)?;

        let reader = BufReader::new(file);
        let mut events = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    warn!("Failed to read line {} in events file: {}", line_num, e);
                    continue;
                }
            };

            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<ExecutionEvent>(&line) {
                Ok(event) => events.push(event),
                Err(e) => {
                    warn!(
                        "Failed to parse event at line {} in {}: {}",
                        line_num,
                        path.display(),
                        e
                    );
                    continue;
                }
            }
        }

        Ok(events)
    }

    /// Load events with pagination
    pub fn load_events_paginated(
        &self,
        task_id: Uuid,
        run_id: Uuid,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<ExecutionEvent>, bool)> {
        let path = self.events_path(task_id, run_id);

        if !path.exists() {
            return Ok((Vec::new(), false));
        }

        let file = File::open(&path).map_err(ExecutorError::from)?;

        let reader = BufReader::new(file);
        let mut events = Vec::new();
        let mut total_count = 0;

        for (line_num, line) in reader.lines().enumerate() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            if line.trim().is_empty() {
                continue;
            }

            // Skip until offset
            if total_count < offset {
                total_count += 1;
                continue;
            }

            // Stop if we've collected enough
            if events.len() >= limit {
                total_count += 1;
                continue; // Keep counting for has_more
            }

            match serde_json::from_str::<ExecutionEvent>(&line) {
                Ok(event) => {
                    events.push(event);
                    total_count += 1;
                }
                Err(e) => {
                    warn!("Failed to parse event at line {}: {}", line_num, e);
                    total_count += 1;
                }
            }
        }

        let has_more = total_count > offset + events.len();
        Ok((events, has_more))
    }

    /// Load events with pagination and filters
    pub fn load_events_filtered_paginated(
        &self,
        task_id: Uuid,
        run_id: Uuid,
        offset: usize,
        limit: usize,
        event_type: Option<&str>,
        agent_event_type: Option<&str>,
    ) -> Result<(Vec<ExecutionEvent>, bool)> {
        let path = self.events_path(task_id, run_id);

        if !path.exists() {
            return Ok((Vec::new(), false));
        }

        let file = File::open(&path).map_err(ExecutorError::from)?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        let mut matched_count = 0;

        let event_type = event_type.map(|t| t.to_lowercase());
        let agent_event_type = agent_event_type.map(|t| t.to_lowercase());

        for (line_num, line) in reader.lines().enumerate() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            if line.trim().is_empty() {
                continue;
            }

            let event = match serde_json::from_str::<ExecutionEvent>(&line) {
                Ok(event) => event,
                Err(e) => {
                    warn!("Failed to parse event at line {}: {}", line_num, e);
                    continue;
                }
            };

            if let Some(ref filter) = event_type {
                if !matches_event_type(&event, filter) {
                    continue;
                }
            }

            if let Some(ref filter) = agent_event_type {
                if !matches_agent_event_type(&event, filter) {
                    continue;
                }
            }

            if matched_count < offset {
                matched_count += 1;
                continue;
            }

            if events.len() < limit {
                events.push(event);
            }
            matched_count += 1;
        }

        let has_more = matched_count > offset + events.len();
        Ok((events, has_more))
    }

    /// Delete a run and all its data
    pub fn delete_run(&self, task_id: Uuid, run_id: Uuid) -> Result<()> {
        let dir = self.run_dir(task_id, run_id);

        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(ExecutorError::from)?;
            info!("Deleted run: {}", dir.display());
        }

        Ok(())
    }

    /// Delete all runs for a task
    pub fn delete_task_runs(&self, task_id: Uuid) -> Result<()> {
        let dir = self.task_dir(task_id);

        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(ExecutorError::from)?;
            info!("Deleted all runs for task: {}", task_id);
        }

        Ok(())
    }

    /// Get the event count for a run
    pub fn get_event_count(&self, task_id: Uuid, run_id: Uuid) -> Result<u32> {
        let path = self.events_path(task_id, run_id);

        if !path.exists() {
            return Ok(0);
        }

        let file = File::open(&path).map_err(ExecutorError::from)?;

        let reader = BufReader::new(file);
        let count = reader
            .lines()
            .filter(|l| l.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false))
            .count();

        Ok(count as u32)
    }

    // ============ Message Persistence ============

    /// Append a chat message to a run's message log
    pub fn append_message(&self, task_id: Uuid, run_id: Uuid, message: &ChatMessage) -> Result<()> {
        self.ensure_run_dir(task_id, run_id)?;
        let path = self.messages_path(task_id, run_id);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(ExecutorError::from)?;

        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string(message).map_err(|e| {
            ExecutorError::execution_failed(format!("Failed to serialize message: {}", e))
        })?;

        writeln!(writer, "{}", json).map_err(ExecutorError::from)?;
        writer.flush().map_err(ExecutorError::from)?;

        debug!("Appended message {} to run {}", message.id, run_id);
        Ok(())
    }

    /// Load all chat messages for a run
    pub fn load_messages(&self, task_id: Uuid, run_id: Uuid) -> Result<Vec<ChatMessage>> {
        let path = self.messages_path(task_id, run_id);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&path).map_err(ExecutorError::from)?;
        let reader = BufReader::new(file);
        let mut messages = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    warn!("Failed to read line {} in messages file: {}", line_num, e);
                    continue;
                }
            };

            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<ChatMessage>(&line) {
                Ok(msg) => messages.push(msg),
                Err(e) => {
                    warn!(
                        "Failed to parse message at line {} in {}: {}",
                        line_num,
                        path.display(),
                        e
                    );
                    continue;
                }
            }
        }

        debug!("Loaded {} messages for run {}", messages.len(), run_id);
        Ok(messages)
    }

    /// Get the message count for a run
    pub fn get_message_count(&self, task_id: Uuid, run_id: Uuid) -> Result<u32> {
        let path = self.messages_path(task_id, run_id);

        if !path.exists() {
            return Ok(0);
        }

        let file = File::open(&path).map_err(ExecutorError::from)?;
        let reader = BufReader::new(file);
        let count = reader
            .lines()
            .filter(|l| l.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false))
            .count();

        Ok(count as u32)
    }
}

fn matches_event_type(event: &ExecutionEvent, filter: &str) -> bool {
    match (filter, &event.event) {
        ("status_changed", ExecutionEventType::StatusChanged { .. }) => true,
        ("agent_event", ExecutionEventType::AgentEvent { .. }) => true,
        ("session_started", ExecutionEventType::SessionStarted { .. }) => true,
        ("session_ended", ExecutionEventType::SessionEnded { .. }) => true,
        ("progress", ExecutionEventType::Progress { .. }) => true,
        _ => false,
    }
}

fn matches_agent_event_type(event: &ExecutionEvent, filter: &str) -> bool {
    match &event.event {
        ExecutionEventType::AgentEvent { event } => match (filter, event) {
            ("thinking", AgentEvent::Thinking { .. }) => true,
            ("command", AgentEvent::Command { .. }) => true,
            ("file_change", AgentEvent::FileChange { .. }) => true,
            ("tool_call", AgentEvent::ToolCall { .. }) => true,
            ("message", AgentEvent::Message { .. }) => true,
            ("error", AgentEvent::Error { .. }) => true,
            ("completed", AgentEvent::Completed { .. }) => true,
            ("raw_output", AgentEvent::RawOutput { .. }) => true,
            _ => false,
        },
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::AgentEvent;
    use crate::process::AgentType;
    use tempfile::TempDir;

    fn create_test_store() -> (RunStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store = RunStore::new(temp_dir.path());
        (store, temp_dir)
    }

    #[test]
    fn test_save_and_load_run() {
        let (store, _temp) = create_test_store();

        let run = Run::new(
            Uuid::new_v4(),
            AgentType::OpenCode,
            "Test prompt".to_string(),
            "main".to_string(),
        );

        // Save
        store.save_run(&run).unwrap();

        // Load
        let loaded = store.load_run(run.task_id, run.id).unwrap();

        assert_eq!(loaded.id, run.id);
        assert_eq!(loaded.task_id, run.task_id);
        assert_eq!(loaded.prompt, run.prompt);
    }

    #[test]
    fn test_load_run_without_context_metadata_fields() {
        let (store, temp_dir) = create_test_store();
        let task_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();

        let mut run = Run::new(
            task_id,
            AgentType::OpenCode,
            "Legacy payload".to_string(),
            "main".to_string(),
        );
        run.id = run_id;

        let mut run_json = serde_json::to_value(&run).unwrap();
        let metadata = run_json["metadata"].as_object_mut().unwrap();
        metadata.remove("project_id");
        metadata.remove("workspace_id");

        let run_dir = temp_dir
            .path()
            .join("runs")
            .join(task_id.to_string())
            .join(run_id.to_string());
        fs::create_dir_all(&run_dir).unwrap();
        let run_path = run_dir.join("run.json");
        fs::write(&run_path, serde_json::to_vec_pretty(&run_json).unwrap()).unwrap();

        let loaded = store.load_run(task_id, run_id).unwrap();
        assert_eq!(loaded.id, run_id);
        assert_eq!(loaded.metadata.project_id, None);
        assert_eq!(loaded.metadata.workspace_id, None);
    }

    #[test]
    fn test_list_runs() {
        let (store, _temp) = create_test_store();
        let task_id = Uuid::new_v4();

        // Create multiple runs
        for i in 0..3 {
            let run = Run::new(
                task_id,
                AgentType::OpenCode,
                format!("Prompt {}", i),
                "main".to_string(),
            );
            store.save_run(&run).unwrap();
        }

        // List
        let runs = store.list_runs(task_id).unwrap();
        assert_eq!(runs.len(), 3);
    }

    #[test]
    fn test_list_runs_empty_task() {
        let (store, _temp) = create_test_store();
        let task_id = Uuid::new_v4();

        let runs = store.list_runs(task_id).unwrap();
        assert!(runs.is_empty());
    }

    #[test]
    fn test_append_and_load_events() {
        let (store, _temp) = create_test_store();
        let task_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();

        // Create and save run first
        let run = Run::with_id(
            run_id,
            task_id,
            AgentType::OpenCode,
            "Test".to_string(),
            "main".to_string(),
        );
        store.save_run(&run).unwrap();

        // Append events
        for i in 0..5 {
            let event = ExecutionEvent::progress(
                run_id,
                task_id,
                format!("Progress {}", i),
                Some(i as f32 * 20.0),
            );
            store.append_event(task_id, run_id, &event).unwrap();
        }

        // Load events
        let events = store.load_events(task_id, run_id).unwrap();
        assert_eq!(events.len(), 5);
    }

    #[test]
    fn test_load_events_paginated() {
        let (store, _temp) = create_test_store();
        let task_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();

        // Append 10 events
        for i in 0..10 {
            let event = ExecutionEvent::progress(run_id, task_id, format!("Progress {}", i), None);
            store.append_event(task_id, run_id, &event).unwrap();
        }

        // Load first page
        let (events, has_more) = store.load_events_paginated(task_id, run_id, 0, 5).unwrap();
        assert_eq!(events.len(), 5);
        assert!(has_more);

        // Load second page
        let (events, has_more) = store.load_events_paginated(task_id, run_id, 5, 5).unwrap();
        assert_eq!(events.len(), 5);
        assert!(!has_more);
    }

    #[test]
    fn test_load_events_filtered_paginated() {
        let (store, _temp) = create_test_store();
        let task_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();

        let run = Run::with_id(
            run_id,
            task_id,
            AgentType::OpenCode,
            "Test".to_string(),
            "main".to_string(),
        );
        store.save_run(&run).unwrap();

        let progress = ExecutionEvent::progress(run_id, task_id, "Progress".to_string(), None);
        store.append_event(task_id, run_id, &progress).unwrap();

        let agent_event = ExecutionEvent::agent_event(
            run_id,
            task_id,
            AgentEvent::Message {
                content: "Hello".to_string(),
            },
        );
        store.append_event(task_id, run_id, &agent_event).unwrap();

        let (events, has_more) = store
            .load_events_filtered_paginated(task_id, run_id, 0, 10, Some("agent_event"), None)
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(!has_more);

        let (events, has_more) = store
            .load_events_filtered_paginated(
                task_id,
                run_id,
                0,
                10,
                Some("agent_event"),
                Some("message"),
            )
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(!has_more);
    }

    #[test]
    fn test_get_event_count() {
        let (store, _temp) = create_test_store();
        let task_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();

        // Initially empty
        assert_eq!(store.get_event_count(task_id, run_id).unwrap(), 0);

        // Append events
        for i in 0..7 {
            let event = ExecutionEvent::progress(run_id, task_id, format!("P{}", i), None);
            store.append_event(task_id, run_id, &event).unwrap();
        }

        assert_eq!(store.get_event_count(task_id, run_id).unwrap(), 7);
    }

    #[test]
    fn test_delete_run() {
        let (store, _temp) = create_test_store();
        let task_id = Uuid::new_v4();

        let run = Run::new(
            task_id,
            AgentType::OpenCode,
            "Test".to_string(),
            "main".to_string(),
        );
        store.save_run(&run).unwrap();

        // Verify exists
        assert!(store.load_run(task_id, run.id).is_ok());

        // Delete
        store.delete_run(task_id, run.id).unwrap();

        // Verify deleted
        assert!(store.load_run(task_id, run.id).is_err());
    }

    #[test]
    fn test_delete_task_runs() {
        let (store, _temp) = create_test_store();
        let task_id = Uuid::new_v4();

        // Create runs
        for _ in 0..3 {
            let run = Run::new(
                task_id,
                AgentType::OpenCode,
                "Test".to_string(),
                "main".to_string(),
            );
            store.save_run(&run).unwrap();
        }

        assert_eq!(store.list_runs(task_id).unwrap().len(), 3);

        // Delete all
        store.delete_task_runs(task_id).unwrap();

        assert!(store.list_runs(task_id).unwrap().is_empty());
    }
}
