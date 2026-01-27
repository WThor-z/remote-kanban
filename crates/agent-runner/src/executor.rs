//! Task executor - orchestrates task execution in isolated worktrees

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use git_worktree::{WorktreeConfig, WorktreeManager};

use crate::error::{ExecutorError, Result};
use crate::event::{ExecutionEvent, ExecutionStatus};
use crate::process::AgentType;
use crate::session::{ExecutionSession, SessionState};

/// Configuration for the task executor
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Path to the repository
    pub repo_path: PathBuf,
    /// Worktree configuration
    pub worktree_config: WorktreeConfig,
    /// Whether to auto-cleanup worktrees on completion
    pub auto_cleanup: bool,
    /// Whether to delete branches on cleanup
    pub delete_branches: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            repo_path: PathBuf::from("."),
            worktree_config: WorktreeConfig::default(),
            auto_cleanup: false, // Manual cleanup by default for safety
            delete_branches: false,
        }
    }
}

/// Request to start task execution
#[derive(Debug, Clone)]
pub struct ExecuteRequest {
    /// Task ID
    pub task_id: Uuid,
    /// Agent type to use
    pub agent_type: String,
    /// Base branch to create worktree from
    pub base_branch: String,
    /// The task prompt/description
    pub prompt: String,
}

/// Task executor that manages execution sessions
pub struct TaskExecutor {
    /// Configuration
    config: ExecutorConfig,
    /// Worktree manager
    worktree_manager: Arc<WorktreeManager>,
    /// Active sessions by session ID
    sessions: Arc<RwLock<HashMap<Uuid, Arc<RwLock<ExecutionSession>>>>>,
    /// Sessions by task ID (for lookup)
    task_sessions: Arc<RwLock<HashMap<Uuid, Uuid>>>,
}

impl TaskExecutor {
    /// Create a new task executor
    pub async fn new(config: ExecutorConfig) -> Result<Self> {
        let worktree_manager = WorktreeManager::with_config(
            &config.repo_path,
            config.worktree_config.clone(),
        )
        .await?;

        Ok(Self {
            config,
            worktree_manager: Arc::new(worktree_manager),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            task_sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Execute a task
    ///
    /// Returns a receiver for execution events
    pub async fn execute(
        &self,
        request: ExecuteRequest,
    ) -> Result<(Uuid, mpsc::Receiver<ExecutionEvent>)> {
        // Check if task already has an active session
        {
            let task_sessions = self.task_sessions.read().await;
            if let Some(session_id) = task_sessions.get(&request.task_id) {
                let sessions = self.sessions.read().await;
                if let Some(session) = sessions.get(session_id) {
                    let state = session.read().await.state().await;
                    if !state.is_terminal() {
                        return Err(ExecutorError::SessionExists {
                            task_id: request.task_id.to_string(),
                        });
                    }
                }
            }
        }

        // Parse agent type
        let agent_type = AgentType::from_str(&request.agent_type)?;

        // Create session
        let mut session = ExecutionSession::new(
            request.task_id,
            agent_type,
            request.prompt.clone(),
            request.base_branch.clone(),
        );

        let session_id = session.id;
        info!("Creating execution session {} for task {}", session_id, request.task_id);

        // Update status
        session.update_status(ExecutionStatus::CreatingWorktree).await;
        session.emit_progress("Creating isolated worktree...".to_string(), Some(0.1)).await;

        // Create worktree
        let task_id_str = request.task_id.to_string();
        let worktree = self
            .worktree_manager
            .create(&task_id_str, &request.base_branch)
            .await?;

        info!(
            "Created worktree at {:?} on branch {}",
            worktree.path, worktree.branch
        );

        session.set_worktree(worktree);
        session.emit_progress("Worktree created, starting agent...".to_string(), Some(0.3)).await;

        // Take the event receiver before moving session into Arc
        let event_rx = session.take_event_receiver().ok_or_else(|| {
            ExecutorError::spawn_failed("Failed to get event receiver")
        })?;

        // Store session
        let session = Arc::new(RwLock::new(session));
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id, Arc::clone(&session));
        }
        {
            let mut task_sessions = self.task_sessions.write().await;
            task_sessions.insert(request.task_id, session_id);
        }

        // Start execution in background
        let session_clone = Arc::clone(&session);
        let worktree_manager = Arc::clone(&self.worktree_manager);
        let auto_cleanup = self.config.auto_cleanup;
        let delete_branches = self.config.delete_branches;

        tokio::spawn(async move {
            let result = run_session(session_clone.clone()).await;

            match result {
                Ok(exit_code) => {
                    let session = session_clone.read().await;
                    session.complete(exit_code).await;
                    info!("Session completed with exit code {}", exit_code);
                }
                Err(e) => {
                    let session = session_clone.read().await;
                    session.fail(e.to_string()).await;
                    error!("Session failed: {}", e);
                }
            }

            // Cleanup if configured
            if auto_cleanup {
                let session = session_clone.read().await;
                if let Some(worktree) = &session.worktree {
                    if let Err(e) = worktree_manager
                        .remove(&worktree.path, true, delete_branches)
                        .await
                    {
                        warn!("Failed to cleanup worktree: {}", e);
                    }
                }
            }
        });

        Ok((session_id, event_rx))
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: Uuid) -> Option<Arc<RwLock<ExecutionSession>>> {
        let sessions = self.sessions.read().await;
        sessions.get(&session_id).cloned()
    }

    /// Get a session by task ID
    pub async fn get_session_by_task(&self, task_id: Uuid) -> Option<Arc<RwLock<ExecutionSession>>> {
        let task_sessions = self.task_sessions.read().await;
        if let Some(session_id) = task_sessions.get(&task_id) {
            let sessions = self.sessions.read().await;
            return sessions.get(session_id).cloned();
        }
        None
    }

    /// Cancel a session
    pub async fn cancel_session(&self, session_id: Uuid) -> Result<()> {
        let session = self
            .get_session(session_id)
            .await
            .ok_or_else(|| ExecutorError::SessionNotFound {
                session_id: session_id.to_string(),
            })?;

        let session = session.read().await;
        session.cancel().await;

        // Note: The actual process killing would need to be handled
        // by storing the OutputReaderHandle in the session

        Ok(())
    }

    /// List all active sessions
    pub async fn list_sessions(&self) -> Vec<(Uuid, Uuid, SessionState)> {
        let sessions = self.sessions.read().await;
        let mut result = Vec::new();

        for (session_id, session) in sessions.iter() {
            let session = session.read().await;
            result.push((*session_id, session.task_id, session.state().await));
        }

        result
    }

    /// Cleanup a session's worktree
    pub async fn cleanup_session(&self, session_id: Uuid, force: bool) -> Result<()> {
        let session = self
            .get_session(session_id)
            .await
            .ok_or_else(|| ExecutorError::SessionNotFound {
                session_id: session_id.to_string(),
            })?;

        let session = session.read().await;
        let state = session.state().await;

        if !state.is_terminal() && !force {
            return Err(ExecutorError::SessionAlreadyRunning {
                session_id: session_id.to_string(),
            });
        }

        if let Some(worktree) = &session.worktree {
            self.worktree_manager
                .remove(&worktree.path, force, self.config.delete_branches)
                .await?;
        }

        Ok(())
    }

    /// Get the worktree manager
    pub fn worktree_manager(&self) -> &WorktreeManager {
        &self.worktree_manager
    }
}

/// Run a session (internal)
async fn run_session(session: Arc<RwLock<ExecutionSession>>) -> Result<i32> {
    // Start the agent process
    let handle = {
        let mut session = session.write().await;
        session.start().await?
    };

    // Wait for completion
    let exit_code = handle.wait().await?;

    Ok(exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use git_worktree::WorktreeConfig;

    #[test]
    fn test_executor_config_default() {
        let config = ExecutorConfig::default();
        assert!(!config.auto_cleanup);
        assert!(!config.delete_branches);
    }

    #[test]
    fn test_execute_request() {
        let request = ExecuteRequest {
            task_id: Uuid::new_v4(),
            agent_type: "opencode".to_string(),
            base_branch: "main".to_string(),
            prompt: "Test prompt".to_string(),
        };

        assert!(!request.task_id.is_nil());
    }
}
