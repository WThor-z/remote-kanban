//! Task executor - orchestrates task execution in isolated worktrees

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use git_worktree::{Worktree, WorktreeConfig, WorktreeManager};

use crate::client::{WorkerClient, WorkerClientApi};
use crate::error::{ExecutorError, Result};
use crate::event::{AgentEvent, ExecutionEvent, ExecutionEventType, ExecutionStatus};
use crate::persistence::RunStore;
use crate::process::AgentType;
use crate::run::{Run, RunSummary};
use crate::session::{ExecutionSession, SessionState};

pub trait WorktreeManagerApi: Send + Sync {
    fn create(
        &self,
        task_id: String,
        base_branch: String,
    ) -> Pin<Box<dyn Future<Output = Result<Worktree>> + Send + '_>>;

    fn remove(
        &self,
        path: PathBuf,
        force: bool,
        delete_branches: bool,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
}

impl WorktreeManagerApi for WorktreeManager {
    fn create(
        &self,
        task_id: String,
        base_branch: String,
    ) -> Pin<Box<dyn Future<Output = Result<Worktree>> + Send + '_>> {
        Box::pin(async move {
            self.create(&task_id, &base_branch)
                .await
                .map_err(ExecutorError::from)
        })
    }

    fn remove(
        &self,
        path: PathBuf,
        force: bool,
        delete_branches: bool,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            self.remove(&path, force, delete_branches)
                .await
                .map_err(ExecutorError::from)
        })
    }
}

/// Configuration for the task executor
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Path to the data directory (for runs/events)
    pub data_dir: PathBuf,
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
            data_dir: PathBuf::from(".vk-data"),
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
    worktree_manager: Arc<dyn WorktreeManagerApi>,
    /// Worker client
    worker_client: Arc<dyn WorkerClientApi>,
    /// Run store for persistence
    run_store: Arc<RunStore>,
    /// Active sessions by session ID
    sessions: Arc<RwLock<HashMap<Uuid, Arc<RwLock<ExecutionSession>>>>>,
    /// Sessions by task ID (for lookup)
    task_sessions: Arc<RwLock<HashMap<Uuid, Uuid>>>,
    /// Active runs by session ID
    active_runs: Arc<RwLock<HashMap<Uuid, Arc<RwLock<Run>>>>>,
}

impl TaskExecutor {
    /// Create a new task executor
    pub async fn new(config: ExecutorConfig) -> Result<Self> {
        let worktree_manager =
            WorktreeManager::with_config(&config.repo_path, config.worktree_config.clone()).await?;

        let worker_url = std::env::var("AGENT_WORKER_URL")
            .unwrap_or_else(|_| "http://localhost:4000".to_string());
        let worker_client = WorkerClient::new(worker_url);

        Ok(Self::new_with_dependencies(
            config,
            Arc::new(worktree_manager),
            Arc::new(worker_client),
        ))
    }

    fn new_with_dependencies(
        config: ExecutorConfig,
        worktree_manager: Arc<dyn WorktreeManagerApi>,
        worker_client: Arc<dyn WorkerClientApi>,
    ) -> Self {
        let run_store = Arc::new(RunStore::new(&config.data_dir));

        Self {
            config,
            worktree_manager,
            worker_client,
            run_store,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            task_sessions: Arc::new(RwLock::new(HashMap::new())),
            active_runs: Arc::new(RwLock::new(HashMap::new())),
        }
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
        info!(
            "Creating execution session {} for task {}",
            session_id, request.task_id
        );

        // Update status
        session
            .update_status(ExecutionStatus::CreatingWorktree)
            .await;
        session
            .emit_progress("Creating isolated worktree...".to_string(), Some(0.1))
            .await;

        // Create worktree
        let task_id_str = request.task_id.to_string();
        let worktree = self
            .worktree_manager
            .create(task_id_str.clone(), request.base_branch.clone())
            .await?;

        info!(
            "Created worktree at {:?} on branch {}",
            worktree.path, worktree.branch
        );

        session.set_worktree(worktree);
        session
            .emit_progress("Worktree created, starting agent...".to_string(), Some(0.3))
            .await;

        let worktree =
            session
                .worktree
                .as_ref()
                .ok_or_else(|| ExecutorError::WorktreePathNotFound {
                    path: PathBuf::from("(not set)"),
                })?;

        let worktree_path = worktree
            .path
            .strip_prefix(&self.config.repo_path)
            .ok()
            .map(|p| p.to_path_buf())
            .or_else(|| Some(worktree.path.clone()));

        let task_id = request.task_id;
        let mut run = Run::with_id(
            session_id,
            task_id,
            agent_type,
            request.prompt.clone(),
            request.base_branch.clone(),
        );
        run.created_at = session.created_at;
        run.worktree_branch = Some(worktree.branch.clone());
        run.worktree_path = worktree_path;
        run.status = ExecutionStatus::CreatingWorktree;
        run.events_path = Some(
            PathBuf::from("runs")
                .join(task_id.to_string())
                .join(session_id.to_string())
                .join("events.jsonl"),
        );

        self.run_store.save_run(&run)?;

        let run_handle = Arc::new(RwLock::new(run));
        {
            let mut active_runs = self.active_runs.write().await;
            active_runs.insert(session_id, Arc::clone(&run_handle));
        }

        // Take the event receiver before moving session into Arc
        let event_rx = session
            .take_event_receiver()
            .ok_or_else(|| ExecutorError::spawn_failed("Failed to get event receiver"))?;

        let (forward_tx, forward_rx) = mpsc::channel(1000);

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
        let worker_client = Arc::clone(&self.worker_client);
        let auto_cleanup = self.config.auto_cleanup;
        let delete_branches = self.config.delete_branches;

        let run_store = Arc::clone(&self.run_store);
        let run_handle = Arc::clone(&run_handle);
        let active_runs = Arc::clone(&self.active_runs);

        tokio::spawn(async move {
            let result = run_session(session_clone.clone(), worker_client).await;

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
                        .remove(worktree.path.clone(), true, delete_branches)
                        .await
                    {
                        warn!("Failed to cleanup worktree: {}", e);
                    }
                }
            }
        });

        let mut event_rx = event_rx;
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                if let Err(e) = run_store.append_event(task_id, session_id, &event) {
                    warn!("Failed to persist event for session {}: {}", session_id, e);
                }

                let run_snapshot = {
                    let mut run = run_handle.write().await;
                    update_run_from_event(&mut run, &event);
                    run.increment_event_count();
                    run.clone()
                };

                if let Err(e) = run_store.save_run(&run_snapshot) {
                    warn!(
                        "Failed to persist run metadata for session {}: {}",
                        session_id, e
                    );
                }

                let should_close = matches!(event.event, ExecutionEventType::SessionEnded { .. });

                if forward_tx.send(event).await.is_err() {
                    break;
                }

                if should_close {
                    break;
                }
            }

            active_runs.write().await.remove(&session_id);
        });

        Ok((session_id, forward_rx))
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: Uuid) -> Option<Arc<RwLock<ExecutionSession>>> {
        let sessions = self.sessions.read().await;
        sessions.get(&session_id).cloned()
    }

    /// Get a session by task ID
    pub async fn get_session_by_task(
        &self,
        task_id: Uuid,
    ) -> Option<Arc<RwLock<ExecutionSession>>> {
        let task_sessions = self.task_sessions.read().await;
        if let Some(session_id) = task_sessions.get(&task_id) {
            let sessions = self.sessions.read().await;
            return sessions.get(session_id).cloned();
        }
        None
    }

    /// List runs for a task
    pub fn list_runs(&self, task_id: Uuid) -> Result<Vec<RunSummary>> {
        self.run_store.list_runs(task_id)
    }

    /// Load run events with pagination and filters
    pub fn load_run_events(
        &self,
        task_id: Uuid,
        run_id: Uuid,
        offset: usize,
        limit: usize,
        event_type: Option<String>,
        agent_event_type: Option<String>,
    ) -> Result<(Vec<ExecutionEvent>, bool)> {
        self.run_store.load_events_filtered_paginated(
            task_id,
            run_id,
            offset,
            limit,
            event_type.as_deref(),
            agent_event_type.as_deref(),
        )
    }

    /// Cancel a session
    pub async fn cancel_session(&self, session_id: Uuid) -> Result<()> {
        let session =
            self.get_session(session_id)
                .await
                .ok_or_else(|| ExecutorError::SessionNotFound {
                    session_id: session_id.to_string(),
                })?;

        let task_id = {
            let session = session.read().await;
            session.cancel().await;
            session.task_id
        };

        // Stop on worker (best effort)
        if let Err(e) = self.worker_client.stop(task_id.to_string()).await {
            warn!("Failed to stop remote task: {}", e);
        }

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
        let session =
            self.get_session(session_id)
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
                .remove(worktree.path.clone(), force, self.config.delete_branches)
                .await?;
        }

        Ok(())
    }

    /// Get the worktree manager
    pub fn worktree_manager(&self) -> &dyn WorktreeManagerApi {
        self.worktree_manager.as_ref()
    }

    /// Get the run store for persistence operations
    pub fn run_store(&self) -> &RunStore {
        &self.run_store
    }

    /// Send input to a running session
    pub async fn send_input(&self, task_id: Uuid, content: String) -> Result<()> {
        let session = self.get_session_by_task(task_id).await.ok_or_else(|| {
            ExecutorError::SessionNotFoundForTask {
                task_id: task_id.to_string(),
            }
        })?;

        let session = session.read().await;
        let state = session.state().await;
        if !state.is_running() {
            return Err(ExecutorError::SessionNotRunning {
                session_id: session.id.to_string(),
            });
        }

        self.worker_client
            .send_input(task_id.to_string(), content)
            .await
    }
}

fn update_run_from_event(run: &mut Run, event: &ExecutionEvent) {
    match &event.event {
        ExecutionEventType::StatusChanged { new_status, .. } => {
            run.status = *new_status;
        }
        ExecutionEventType::AgentEvent { event: agent_event } => match agent_event {
            AgentEvent::Thinking { .. } => {
                run.metadata.thinking_count = run.metadata.thinking_count.saturating_add(1);
            }
            AgentEvent::Command { .. } => {
                run.metadata.commands_executed = run.metadata.commands_executed.saturating_add(1);
            }
            AgentEvent::FileChange { path, .. } => {
                if !run.metadata.files_modified.contains(path) {
                    run.metadata.files_modified.push(path.clone());
                }
            }
            AgentEvent::ToolCall { .. } => {
                run.metadata.tools_called = run.metadata.tools_called.saturating_add(1);
            }
            AgentEvent::Message { .. } => {
                run.metadata.message_count = run.metadata.message_count.saturating_add(1);
            }
            AgentEvent::Error { message, .. } => {
                run.metadata.error_count = run.metadata.error_count.saturating_add(1);
                run.error = Some(message.clone());
            }
            AgentEvent::Completed { summary, .. } => {
                if let Some(summary) = summary {
                    run.summary = Some(summary.clone());
                }
            }
            AgentEvent::RawOutput { .. } => {}
        },
        ExecutionEventType::SessionStarted {
            worktree_path,
            branch,
        } => {
            run.started_at = Some(event.timestamp);
            if !worktree_path.is_empty() {
                run.worktree_path = Some(PathBuf::from(worktree_path));
            }
            if !branch.is_empty() {
                run.worktree_branch = Some(branch.clone());
            }
        }
        ExecutionEventType::SessionEnded {
            status,
            duration_ms,
        } => {
            run.ended_at = Some(event.timestamp);
            run.status = *status;
            run.duration_ms = Some(*duration_ms);
        }
        ExecutionEventType::Progress { .. } => {}
    }
}

/// Run a session (internal)
async fn run_session(
    session: Arc<RwLock<ExecutionSession>>,
    worker_client: Arc<dyn WorkerClientApi>,
) -> Result<i32> {
    // Start session (updates state)
    {
        let mut session = session.write().await;
        session.start().await?;
    }

    // Get info needed for execution
    let (task_id, prompt, worktree_path, agent_type, event_tx) = {
        let session = session.read().await;
        let worktree_path = session
            .worktree_path()
            .ok_or(ExecutorError::WorktreePathNotFound {
                path: PathBuf::from(""),
            })?
            .clone();
        (
            session.task_id,
            session.prompt.clone(),
            worktree_path,
            session.agent_type,
            session.agent_event_sender(),
        )
    };

    // Execute via Worker
    match worker_client
        .execute(
            task_id.to_string(),
            prompt,
            worktree_path,
            agent_type,
            event_tx,
        )
        .await
    {
        Ok(_) => Ok(0),
        Err(e) => {
            error!("Execution failed: {}", e);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::WorkerClientApi;
    use crate::event::AgentEvent;
    use git_worktree::{Worktree, WorktreeStatus};
    use tokio::sync::Mutex;

    struct MockWorktreeManager;

    impl WorktreeManagerApi for MockWorktreeManager {
        fn create(
            &self,
            task_id: String,
            _base_branch: String,
        ) -> Pin<Box<dyn Future<Output = Result<Worktree>> + Send + '_>> {
            let worktree = Worktree {
                path: PathBuf::from("mock-worktree"),
                branch: format!("task/{}", task_id),
                head: "mock-head".to_string(),
                status: WorktreeStatus::Active,
                is_main: false,
            };
            Box::pin(async move { Ok(worktree) })
        }

        fn remove(
            &self,
            _path: PathBuf,
            _force: bool,
            _delete_branches: bool,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
            Box::pin(async move { Ok(()) })
        }
    }

    #[derive(Default)]
    struct MockWorkerClient {
        inputs: Arc<Mutex<Vec<(String, String)>>>,
    }

    impl WorkerClientApi for MockWorkerClient {
        fn execute(
            &self,
            _task_id: String,
            _prompt: String,
            _cwd: PathBuf,
            _agent_type: AgentType,
            _event_tx: mpsc::Sender<AgentEvent>,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
            Box::pin(async move { Ok(()) })
        }

        fn stop(&self, _task_id: String) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
            Box::pin(async move { Ok(()) })
        }

        fn send_input(
            &self,
            task_id: String,
            content: String,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
            let inputs = Arc::clone(&self.inputs);
            Box::pin(async move {
                let mut guard = inputs.lock().await;
                guard.push((task_id, content));
                Ok(())
            })
        }
    }

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

    #[tokio::test]
    async fn send_input_returns_error_when_no_session() {
        let worktree_manager: Arc<dyn WorktreeManagerApi> = Arc::new(MockWorktreeManager);
        let worker_client: Arc<dyn WorkerClientApi> = Arc::new(MockWorkerClient::default());
        let executor = TaskExecutor::new_with_dependencies(
            ExecutorConfig::default(),
            worktree_manager,
            worker_client,
        );

        let task_id = Uuid::new_v4();
        let result = executor.send_input(task_id, "hello".to_string()).await;

        match result {
            Err(ExecutorError::SessionNotFoundForTask {
                task_id: err_task_id,
            }) => {
                assert_eq!(err_task_id, task_id.to_string());
            }
            other => panic!("Unexpected result: {:?}", other),
        }
    }

    #[tokio::test]
    async fn send_input_returns_error_when_session_not_running() {
        let worktree_manager: Arc<dyn WorktreeManagerApi> = Arc::new(MockWorktreeManager);
        let worker_client: Arc<dyn WorkerClientApi> = Arc::new(MockWorkerClient::default());
        let executor = TaskExecutor::new_with_dependencies(
            ExecutorConfig::default(),
            worktree_manager,
            worker_client,
        );

        let task_id = Uuid::new_v4();
        let session = ExecutionSession::new(
            task_id,
            AgentType::OpenCode,
            "prompt".to_string(),
            "main".to_string(),
        );
        let session_id = session.id;
        let session = Arc::new(RwLock::new(session));

        executor
            .sessions
            .write()
            .await
            .insert(session_id, Arc::clone(&session));
        executor
            .task_sessions
            .write()
            .await
            .insert(task_id, session_id);

        let result = executor.send_input(task_id, "input".to_string()).await;

        match result {
            Err(ExecutorError::SessionNotRunning {
                session_id: err_session_id,
            }) => {
                assert_eq!(err_session_id, session_id.to_string());
            }
            other => panic!("Unexpected result: {:?}", other),
        }
    }

    #[tokio::test]
    async fn send_input_for_running_session_is_forwarded_to_worker() {
        let worktree_manager: Arc<dyn WorktreeManagerApi> = Arc::new(MockWorktreeManager);
        let mock_worker = Arc::new(MockWorkerClient::default());
        let worker_client: Arc<dyn WorkerClientApi> = mock_worker.clone();
        let executor = TaskExecutor::new_with_dependencies(
            ExecutorConfig::default(),
            worktree_manager,
            worker_client,
        );

        let task_id = Uuid::new_v4();
        let mut session = ExecutionSession::new(
            task_id,
            AgentType::OpenCode,
            "prompt".to_string(),
            "main".to_string(),
        );
        session.set_worktree(Worktree {
            path: PathBuf::from("mock-worktree"),
            branch: "task/mock".to_string(),
            head: "mock-head".to_string(),
            status: WorktreeStatus::Active,
            is_main: false,
        });
        session.start().await.expect("session start");

        let session_id = session.id;
        let session = Arc::new(RwLock::new(session));
        executor
            .sessions
            .write()
            .await
            .insert(session_id, Arc::clone(&session));
        executor
            .task_sessions
            .write()
            .await
            .insert(task_id, session_id);

        executor
            .send_input(task_id, "ping".to_string())
            .await
            .expect("send input");

        let inputs = mock_worker.inputs.lock().await;
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].0, task_id.to_string());
        assert_eq!(inputs[0].1, "ping");
    }
}
