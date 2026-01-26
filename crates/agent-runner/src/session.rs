//! Execution session management

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use git_worktree::Worktree;

use crate::error::{ExecutorError, Result};
use crate::event::{AgentEvent, ExecutionEvent, ExecutionStatus};
use crate::process::{AgentConfig, AgentProcess, AgentType, OutputReaderHandle};

/// State of an execution session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionState {
    /// Session is pending, not yet started
    Pending,
    /// Session is initializing (creating worktree, etc.)
    Initializing,
    /// Session is running with the agent process
    Running {
        pid: Option<u32>,
        started_at: DateTime<Utc>,
    },
    /// Session is paused
    Paused,
    /// Session completed
    Completed {
        exit_code: i32,
        duration_ms: u64,
    },
    /// Session failed
    Failed {
        error: String,
        duration_ms: u64,
    },
    /// Session was cancelled
    Cancelled {
        duration_ms: u64,
    },
}

impl SessionState {
    /// Check if the session is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed { .. } | Self::Failed { .. } | Self::Cancelled { .. }
        )
    }

    /// Check if the session is running
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }
}

/// An execution session for a task
#[derive(Debug)]
pub struct ExecutionSession {
    /// Unique session ID
    pub id: Uuid,
    /// Task ID
    pub task_id: Uuid,
    /// Agent type
    pub agent_type: AgentType,
    /// The prompt being executed
    pub prompt: String,
    /// Base branch
    pub base_branch: String,
    /// Worktree info (set after creation)
    pub worktree: Option<Worktree>,
    /// Current state
    state: Arc<RwLock<SessionState>>,
    /// Current status
    status: Arc<RwLock<ExecutionStatus>>,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When execution started
    pub started_at: Option<Instant>,
    /// Event sender
    event_tx: mpsc::Sender<ExecutionEvent>,
    /// Event receiver (for external consumers)
    event_rx: Option<mpsc::Receiver<ExecutionEvent>>,
    /// Agent event sender (internal)
    agent_event_tx: mpsc::Sender<AgentEvent>,
    /// Agent event receiver (internal)
    agent_event_rx: Option<mpsc::Receiver<AgentEvent>>,
}

impl ExecutionSession {
    /// Create a new execution session
    pub fn new(
        task_id: Uuid,
        agent_type: AgentType,
        prompt: String,
        base_branch: String,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::channel(1000);
        let (agent_event_tx, agent_event_rx) = mpsc::channel(1000);

        Self {
            id: Uuid::new_v4(),
            task_id,
            agent_type,
            prompt,
            base_branch,
            worktree: None,
            state: Arc::new(RwLock::new(SessionState::Pending)),
            status: Arc::new(RwLock::new(ExecutionStatus::Initializing)),
            created_at: Utc::now(),
            started_at: None,
            event_tx,
            event_rx: Some(event_rx),
            agent_event_tx,
            agent_event_rx: Some(agent_event_rx),
        }
    }

    /// Get the current state
    pub async fn state(&self) -> SessionState {
        self.state.read().await.clone()
    }

    /// Get the current status
    pub async fn status(&self) -> ExecutionStatus {
        *self.status.read().await
    }

    /// Set the worktree
    pub fn set_worktree(&mut self, worktree: Worktree) {
        self.worktree = Some(worktree);
    }

    /// Get the worktree path
    pub fn worktree_path(&self) -> Option<&PathBuf> {
        self.worktree.as_ref().map(|w| &w.path)
    }

    /// Take the event receiver (can only be called once)
    pub fn take_event_receiver(&mut self) -> Option<mpsc::Receiver<ExecutionEvent>> {
        self.event_rx.take()
    }

    /// Get event sender for cloning
    pub fn event_sender(&self) -> mpsc::Sender<ExecutionEvent> {
        self.event_tx.clone()
    }

    /// Update the status and emit an event
    pub async fn update_status(&self, new_status: ExecutionStatus) {
        let old_status = {
            let mut status = self.status.write().await;
            let old = *status;
            *status = new_status;
            old
        };

        if old_status != new_status {
            let event = ExecutionEvent::status_changed(
                self.id,
                self.task_id,
                old_status,
                new_status,
            );
            let _ = self.event_tx.send(event).await;
        }
    }

    /// Emit a progress event
    pub async fn emit_progress(&self, message: String, percentage: Option<f32>) {
        let event = ExecutionEvent::progress(self.id, self.task_id, message, percentage);
        let _ = self.event_tx.send(event).await;
    }

    /// Start the session (spawn the agent process)
    pub async fn start(&mut self) -> Result<OutputReaderHandle> {
        // Check state
        {
            let state = self.state.read().await;
            if !matches!(*state, SessionState::Pending | SessionState::Initializing) {
                return Err(ExecutorError::SessionAlreadyRunning {
                    session_id: self.id.to_string(),
                });
            }
        }

        // Get worktree path
        let worktree_path = self
            .worktree_path()
            .ok_or_else(|| ExecutorError::WorktreePathNotFound {
                path: PathBuf::from("(not set)"),
            })?
            .clone();

        // Update state
        {
            let mut state = self.state.write().await;
            *state = SessionState::Initializing;
        }
        self.update_status(ExecutionStatus::Starting).await;

        // Prepare config
        let config = AgentConfig {
            agent_type: self.agent_type,
            working_dir: worktree_path.clone(),
            prompt: self.prompt.clone(),
            env: vec![],
            timeout_seconds: 0,
        };

        // Spawn the agent
        let process = AgentProcess::spawn(config, self.agent_event_tx.clone()).await?;
        let handle = process.start_output_reader().await?;

        // Update state to running
        let started_at = Utc::now();
        self.started_at = Some(Instant::now());
        {
            let mut state = self.state.write().await;
            *state = SessionState::Running {
                pid: handle.pid(),
                started_at,
            };
        }
        self.update_status(ExecutionStatus::Running).await;

        // Emit session started event
        let event = ExecutionEvent::session_started(
            self.id,
            self.task_id,
            worktree_path.to_string_lossy().to_string(),
            self.worktree.as_ref().map(|w| w.branch.clone()).unwrap_or_default(),
        );
        let _ = self.event_tx.send(event).await;

        // Start forwarding agent events
        self.start_event_forwarder();

        Ok(handle)
    }

    /// Start forwarding agent events to the main event stream
    fn start_event_forwarder(&mut self) {
        let mut agent_rx = match self.agent_event_rx.take() {
            Some(rx) => rx,
            None => return,
        };

        let event_tx = self.event_tx.clone();
        let session_id = self.id;
        let task_id = self.task_id;

        tokio::spawn(async move {
            while let Some(agent_event) = agent_rx.recv().await {
                let event = ExecutionEvent::agent_event(session_id, task_id, agent_event);
                if event_tx.send(event).await.is_err() {
                    break;
                }
            }
        });
    }

    /// Mark the session as completed
    pub async fn complete(&self, exit_code: i32) {
        let duration_ms = self
            .started_at
            .map(|s| s.elapsed().as_millis() as u64)
            .unwrap_or(0);

        {
            let mut state = self.state.write().await;
            *state = SessionState::Completed {
                exit_code,
                duration_ms,
            };
        }

        let status = if exit_code == 0 {
            ExecutionStatus::Completed
        } else {
            ExecutionStatus::Failed
        };
        self.update_status(status).await;

        // Emit session ended event
        let event = ExecutionEvent::session_ended(self.id, self.task_id, status, duration_ms);
        let _ = self.event_tx.send(event).await;
    }

    /// Mark the session as failed
    pub async fn fail(&self, error: String) {
        let duration_ms = self
            .started_at
            .map(|s| s.elapsed().as_millis() as u64)
            .unwrap_or(0);

        {
            let mut state = self.state.write().await;
            *state = SessionState::Failed {
                error: error.clone(),
                duration_ms,
            };
        }

        self.update_status(ExecutionStatus::Failed).await;

        // Emit session ended event
        let event = ExecutionEvent::session_ended(
            self.id,
            self.task_id,
            ExecutionStatus::Failed,
            duration_ms,
        );
        let _ = self.event_tx.send(event).await;
    }

    /// Cancel the session
    pub async fn cancel(&self) {
        let duration_ms = self
            .started_at
            .map(|s| s.elapsed().as_millis() as u64)
            .unwrap_or(0);

        {
            let mut state = self.state.write().await;
            *state = SessionState::Cancelled { duration_ms };
        }

        self.update_status(ExecutionStatus::Cancelled).await;

        // Emit session ended event
        let event = ExecutionEvent::session_ended(
            self.id,
            self.task_id,
            ExecutionStatus::Cancelled,
            duration_ms,
        );
        let _ = self.event_tx.send(event).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let session = ExecutionSession::new(
            Uuid::new_v4(),
            AgentType::OpenCode,
            "Test prompt".to_string(),
            "main".to_string(),
        );

        assert!(!session.id.is_nil());
        assert!(matches!(session.state().await, SessionState::Pending));
        assert_eq!(session.status().await, ExecutionStatus::Initializing);
    }

    #[tokio::test]
    async fn test_status_update() {
        let session = ExecutionSession::new(
            Uuid::new_v4(),
            AgentType::OpenCode,
            "Test".to_string(),
            "main".to_string(),
        );

        session.update_status(ExecutionStatus::Running).await;
        assert_eq!(session.status().await, ExecutionStatus::Running);
    }
}
