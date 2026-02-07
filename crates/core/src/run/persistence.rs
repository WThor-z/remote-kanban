use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use tracing::{debug, info, warn};
use uuid::Uuid;

use super::event::ExecutionEvent;
use super::run_model::RunSummary;
use crate::run::{ChatMessage, Run};
use crate::{Error, Result};

type IOResult<T> = std::io::Result<T>;

pub struct RunStore {
    base_dir: PathBuf,
}

impl RunStore {
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        Self {
            base_dir: data_dir.as_ref().join("runs"),
        }
    }

    fn task_dir(&self, task_id: Uuid) -> PathBuf {
        self.base_dir.join(task_id.to_string())
    }

    fn run_dir(&self, task_id: Uuid, run_id: Uuid) -> PathBuf {
        self.task_dir(task_id).join(run_id.to_string())
    }

    fn run_metadata_path(&self, task_id: Uuid, run_id: Uuid) -> PathBuf {
        self.run_dir(task_id, run_id).join("run.json")
    }

    fn events_path(&self, task_id: Uuid, run_id: Uuid) -> PathBuf {
        self.run_dir(task_id, run_id).join("events.jsonl")
    }

    fn messages_path(&self, task_id: Uuid, run_id: Uuid) -> PathBuf {
        self.run_dir(task_id, run_id).join("messages.jsonl")
    }

    fn ensure_run_dir(&self, task_id: Uuid, run_id: Uuid) -> Result<PathBuf> {
        let dir = self.run_dir(task_id, run_id);
        if !dir.exists() {
            fs::create_dir_all(&dir).map_err(Into::<Error>::into)?;
        }
        Ok(dir)
    }

    pub fn save_run(&self, run: &Run) -> Result<()> {
        self.ensure_run_dir(run.task_id, run.id)?;
        let path = self.run_metadata_path(run.task_id, run.id);

        let file = File::create(&path).map_err(Into::<Error>::into)?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, run)
            .map_err(|e| Error::ExecutionFailed(format!("Failed to serialize run: {}", e)))?;

        debug!("Saved run metadata: {}", path.display());
        Ok(())
    }

    pub fn load_run(&self, task_id: Uuid, run_id: Uuid) -> Result<Run> {
        let path = self.run_metadata_path(task_id, run_id);

        let file = File::open(&path).map_err(Into::<Error>::into)?;

        let reader = BufReader::new(file);
        let run: Run = serde_json::from_reader(reader)
            .map_err(|e| Error::ExecutionFailed(format!("Failed to deserialize run: {}", e)))?;

        Ok(run)
    }

    pub fn list_runs(&self, task_id: Uuid) -> Result<Vec<RunSummary>> {
        let task_dir = self.task_dir(task_id);

        if !task_dir.exists() {
            return Ok(Vec::new());
        }

        let mut runs = Vec::new();

        let entries = fs::read_dir(&task_dir).map_err(Into::<Error>::into)?;

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

            let run_id = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => match Uuid::parse_str(name) {
                    Ok(id) => id,
                    Err(_) => continue,
                },
                None => continue,
            };

            match self.load_run(task_id, run_id) {
                Ok(run) => runs.push(RunSummary::from(&run)),
                Err(e) => {
                    warn!("Failed to load run {}: {}", run_id, e);
                    continue;
                }
            }
        }

        runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(runs)
    }

    pub fn append_event(&self, task_id: Uuid, run_id: Uuid, event: &ExecutionEvent) -> Result<()> {
        self.ensure_run_dir(task_id, run_id)?;
        let path = self.events_path(task_id, run_id);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(Into::<Error>::into)?;

        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string(event)
            .map_err(|e| Error::ExecutionFailed(format!("Failed to serialize event: {}", e)))?;

        writeln!(writer, "{}", json).map_err(Into::<Error>::into)?;

        writer.flush().map_err(Into::<Error>::into)?;

        Ok(())
    }

    pub fn load_events(&self, task_id: Uuid, run_id: Uuid) -> Result<Vec<ExecutionEvent>> {
        let path = self.events_path(task_id, run_id);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&path).map_err(Into::<Error>::into)?;

        let reader = BufReader::new(file);
        let mut events = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line: String = match line {
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

        let file = File::open(&path).map_err(Into::<Error>::into)?;

        let reader = BufReader::new(file);
        let mut events = Vec::new();
        let mut total_count = 0;

        for (line_num, line) in reader.lines().enumerate() {
            let line: String = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            if line.trim().is_empty() {
                continue;
            }

            if total_count < offset {
                total_count += 1;
                continue;
            }

            if events.len() >= limit {
                total_count += 1;
                continue;
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

        let file = File::open(&path).map_err(Into::<Error>::into)?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        let mut matched_count = 0;

        let event_type = event_type.map(|t| t.to_lowercase());
        let agent_event_type = agent_event_type.map(|t| t.to_lowercase());

        for (line_num, line) in reader.lines().enumerate() {
            let line: String = match line {
                Ok(l) => l,
                Err(e) => {
                    warn!("Failed to parse event at line {}: {}", line_num, e);
                    continue;
                }
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

    pub fn delete_run(&self, task_id: Uuid, run_id: Uuid) -> Result<()> {
        let dir = self.run_dir(task_id, run_id);

        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(Into::<Error>::into)?;
            info!("Deleted run: {}", dir.display());
        }

        Ok(())
    }

    pub fn delete_task_runs(&self, task_id: Uuid) -> Result<()> {
        let dir = self.task_dir(task_id);

        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(Into::<Error>::into)?;
            info!("Deleted all runs for task: {}", task_id);
        }

        Ok(())
    }

    pub fn get_event_count(&self, task_id: Uuid, run_id: Uuid) -> Result<u32> {
        let path = self.events_path(task_id, run_id);

        if !path.exists() {
            return Ok(0);
        }

        let file = File::open(&path).map_err(Into::<Error>::into)?;

        let reader = BufReader::new(file);
        let count = reader
            .lines()
            .filter(|l: &IOResult<String>| {
                l.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false)
            })
            .count();

        Ok(count as u32)
    }

    pub fn append_message(&self, task_id: Uuid, run_id: Uuid, message: &ChatMessage) -> Result<()> {
        self.ensure_run_dir(task_id, run_id)?;
        let path = self.messages_path(task_id, run_id);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(Into::<Error>::into)?;

        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string(message)
            .map_err(|e| Error::ExecutionFailed(format!("Failed to serialize message: {}", e)))?;

        writeln!(writer, "{}", json).map_err(Into::<Error>::into)?;
        writer.flush().map_err(Into::<Error>::into)?;

        debug!("Appended message {} to run {}", message.id, run_id);
        Ok(())
    }

    pub fn load_messages(&self, task_id: Uuid, run_id: Uuid) -> Result<Vec<ChatMessage>> {
        let path = self.messages_path(task_id, run_id);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&path).map_err(Into::<Error>::into)?;
        let reader = BufReader::new(file);
        let mut messages = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line: String = match line {
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

    pub fn get_message_count(&self, task_id: Uuid, run_id: Uuid) -> Result<u32> {
        let path = self.messages_path(task_id, run_id);

        if !path.exists() {
            return Ok(0);
        }

        let file = File::open(&path).map_err(Into::<Error>::into)?;
        let reader = BufReader::new(file);
        let count = reader
            .lines()
            .filter(|l: &IOResult<String>| {
                l.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false)
            })
            .count();

        Ok(count as u32)
    }
}

fn matches_event_type(event: &ExecutionEvent, filter: &str) -> bool {
    use super::event::ExecutionEventType;

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
    use super::event::{AgentEvent, ExecutionEventType};

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
