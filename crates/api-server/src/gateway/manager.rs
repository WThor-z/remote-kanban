//! Gateway Manager - manages connections to remote Agent Gateways

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};
use tracing::{debug, error, info, warn};

use super::protocol::*;
use vk_core::kanban::{KanbanStore, KanbanTaskStatus};
use vk_core::task::{FileTaskStore, TaskRepository, TaskStatus};

/// Host connection state
pub struct HostConnection {
    pub host_id: String,
    pub capabilities: HostCapabilities,
    pub tx: mpsc::Sender<ServerToGatewayMessage>,
    pub active_tasks: Vec<String>,
    pub last_heartbeat: Instant,
    pub connected_at: Instant,
}

impl HostConnection {
    /// Check if host is available for the given agent type
    pub fn is_available(&self, agent_type: &str) -> bool {
        self.capabilities.agents.contains(&agent_type.to_string())
            && (self.active_tasks.len() as u32) < self.capabilities.max_concurrent
    }
}

/// Task event for broadcasting (includes host info)
#[derive(Debug, Clone, serde::Serialize)]
pub struct BroadcastTaskEvent {
    pub task_id: String,
    pub host_id: String,
    pub event: GatewayAgentEvent,
}

/// Gateway Manager - central hub for gateway connections
pub struct GatewayManager {
    connections: Arc<RwLock<HashMap<String, HostConnection>>>,
    event_tx: broadcast::Sender<BroadcastTaskEvent>,
    /// Pending model requests - maps request_id to response sender
    pending_model_requests: Arc<RwLock<HashMap<String, oneshot::Sender<Vec<ProviderInfo>>>>>,
    /// Task store for updating task status
    task_store: Option<Arc<FileTaskStore>>,
    /// Kanban store for updating kanban board
    kanban_store: Option<Arc<KanbanStore>>,
}

impl GatewayManager {
    /// Create a new Gateway Manager
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            pending_model_requests: Arc::new(RwLock::new(HashMap::new())),
            task_store: None,
            kanban_store: None,
        }
    }

    /// Create a new Gateway Manager with a task store
    pub fn with_task_store(task_store: Arc<FileTaskStore>) -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            pending_model_requests: Arc::new(RwLock::new(HashMap::new())),
            task_store: Some(task_store),
            kanban_store: None,
        }
    }

    /// Create a new Gateway Manager with both task store and kanban store
    pub fn with_stores(task_store: Arc<FileTaskStore>, kanban_store: Arc<KanbanStore>) -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            pending_model_requests: Arc::new(RwLock::new(HashMap::new())),
            task_store: Some(task_store),
            kanban_store: Some(kanban_store),
        }
    }

    /// Set the task store (for use after construction)
    #[allow(dead_code)]
    pub fn set_task_store(&mut self, task_store: Arc<FileTaskStore>) {
        self.task_store = Some(task_store);
    }

    /// Set the kanban store (for use after construction)
    #[allow(dead_code)]
    pub fn set_kanban_store(&mut self, kanban_store: Arc<KanbanStore>) {
        self.kanban_store = Some(kanban_store);
    }

    /// Subscribe to task events (for forwarding to frontend)
    pub fn subscribe(&self) -> broadcast::Receiver<BroadcastTaskEvent> {
        self.event_tx.subscribe()
    }

    /// Register a new host connection
    pub async fn register_host(
        &self,
        host_id: String,
        capabilities: HostCapabilities,
        tx: mpsc::Sender<ServerToGatewayMessage>,
    ) -> bool {
        let mut connections = self.connections.write().await;
        
        if connections.contains_key(&host_id) {
            warn!("Host {} already registered, replacing connection", host_id);
        }

        info!("Registering host: {} ({})", host_id, capabilities.name);
        
        connections.insert(
            host_id.clone(),
            HostConnection {
                host_id,
                capabilities,
                tx,
                active_tasks: Vec::new(),
                last_heartbeat: Instant::now(),
                connected_at: Instant::now(),
            },
        );

        true
    }

    /// Unregister a host (on disconnect)
    pub async fn unregister_host(&self, host_id: &str) {
        let mut connections = self.connections.write().await;
        if connections.remove(host_id).is_some() {
            info!("Host {} unregistered", host_id);
        }
    }

    /// Update heartbeat timestamp for a host
    pub async fn update_heartbeat(&self, host_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(host_id) {
            conn.last_heartbeat = Instant::now();
            debug!("Updated heartbeat for host {}", host_id);
        }
    }

    /// Dispatch a task to an available host
    pub async fn dispatch_task(&self, task: GatewayTaskRequest) -> Result<String, String> {
        let mut connections = self.connections.write().await;

        // Find a suitable host
        let host = connections
            .values_mut()
            .find(|c| c.is_available(&task.agent_type))
            .ok_or_else(|| format!("No available host for agent type: {}", task.agent_type))?;

        let host_id = host.host_id.clone();
        host.active_tasks.push(task.task_id.clone());

        // Send task to gateway
        if let Err(e) = host.tx.send(ServerToGatewayMessage::TaskExecute { task }).await {
            error!("Failed to send task to host {}: {}", host_id, e);
            // Remove from active tasks since send failed
            if let Some(conn) = connections.get_mut(&host_id) {
                conn.active_tasks.retain(|id| id != &host_id);
            }
            return Err(format!("Failed to dispatch task: {}", e));
        }

        info!("Task dispatched to host {}", host_id);
        Ok(host_id)
    }

    /// Handle task started event from gateway
    pub async fn handle_task_started(&self, host_id: &str, task_id: &str, session_id: &str) {
        debug!(
            "Task {} started on host {} (session: {})",
            task_id, host_id, session_id
        );
    }

    /// Handle task event from gateway
    pub async fn handle_task_event(&self, host_id: &str, task_id: &str, event: GatewayAgentEvent) {
        let _ = self.event_tx.send(BroadcastTaskEvent {
            task_id: task_id.to_string(),
            host_id: host_id.to_string(),
            event,
        });
    }

    /// Handle task completed event from gateway
    pub async fn handle_task_completed(&self, host_id: &str, task_id: &str, result: TaskResult) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(host_id) {
            conn.active_tasks.retain(|id| id != task_id);
        }
        drop(connections); // Release lock before async task store operations

        info!(
            "Task {} completed on host {}: success={}",
            task_id, host_id, result.success
        );
        
        // Update task status in TaskStore
        if let Some(task_store) = &self.task_store {
            if let Ok(task_uuid) = uuid::Uuid::parse_str(task_id) {
                match task_store.get(task_uuid).await {
                    Ok(Some(mut task)) => {
                        task.status = TaskStatus::Done;
                        if let Err(e) = task_store.update(task).await {
                            warn!("Failed to update task {} status to Done: {}", task_id, e);
                        } else {
                            info!("Task {} status updated to Done", task_id);
                        }
                    }
                    Ok(None) => {
                        warn!("Task {} not found in task store", task_id);
                    }
                    Err(e) => {
                        warn!("Failed to get task {} from store: {}", task_id, e);
                    }
                }
            }
        }
        
        // Update KanbanStore (this is what the UI actually reads)
        if let Some(kanban_store) = &self.kanban_store {
            if let Err(e) = kanban_store.move_task(task_id, KanbanTaskStatus::Done, None).await {
                warn!("Failed to move kanban task {} to Done: {}", task_id, e);
            } else {
                info!("Kanban task {} moved to Done", task_id);
            }
        }
        
        // Broadcast a synthetic "Completed" event so the event forwarder in executor.rs
        // can detect task completion and emit kanban:sync
        let completed_event = GatewayAgentEvent {
            event_type: GatewayAgentEventType::Completed,
            content: Some(format!("Task completed: success={}", result.success)),
            data: serde_json::json!({ "result": result }),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };
        let _ = self.event_tx.send(BroadcastTaskEvent {
            task_id: task_id.to_string(),
            host_id: host_id.to_string(),
            event: completed_event,
        });
    }

    /// Handle task failed event from gateway
    pub async fn handle_task_failed(&self, host_id: &str, task_id: &str, error: &str) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(host_id) {
            conn.active_tasks.retain(|id| id != task_id);
        }
        drop(connections); // Release lock before async task store operations

        error!("Task {} failed on host {}: {}", task_id, host_id, error);
        
        // Update task status in TaskStore (mark as Todo so user can retry)
        if let Some(task_store) = &self.task_store {
            if let Ok(task_uuid) = uuid::Uuid::parse_str(task_id) {
                match task_store.get(task_uuid).await {
                    Ok(Some(mut task)) => {
                        // Move back to Todo on failure so user can retry
                        task.status = TaskStatus::Todo;
                        if let Err(e) = task_store.update(task).await {
                            warn!("Failed to update task {} status to Todo: {}", task_id, e);
                        } else {
                            info!("Task {} status updated to Todo (failed)", task_id);
                        }
                    }
                    Ok(None) => {
                        warn!("Task {} not found in task store", task_id);
                    }
                    Err(e) => {
                        warn!("Failed to get task {} from store: {}", task_id, e);
                    }
                }
            }
        }
        
        // Update KanbanStore - move back to Todo on failure
        if let Some(kanban_store) = &self.kanban_store {
            if let Err(e) = kanban_store.move_task(task_id, KanbanTaskStatus::Todo, None).await {
                warn!("Failed to move kanban task {} to Todo: {}", task_id, e);
            } else {
                info!("Kanban task {} moved to Todo (failed)", task_id);
            }
        }
        
        // Broadcast a synthetic "Failed" event so the event forwarder in executor.rs
        // can detect task failure and emit kanban:sync
        let failed_event = GatewayAgentEvent {
            event_type: GatewayAgentEventType::Failed,
            content: Some(format!("Task failed: {}", error)),
            data: serde_json::json!({ "error": error }),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };
        let _ = self.event_tx.send(BroadcastTaskEvent {
            task_id: task_id.to_string(),
            host_id: host_id.to_string(),
            event: failed_event,
        });
    }

    /// Abort a running task
    #[allow(dead_code)]
    pub async fn abort_task(&self, task_id: &str) -> Result<(), String> {
        let connections = self.connections.read().await;

        for conn in connections.values() {
            if conn.active_tasks.contains(&task_id.to_string()) {
                conn.tx
                    .send(ServerToGatewayMessage::TaskAbort {
                        task_id: task_id.to_string(),
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                info!("Sent abort for task {} to host {}", task_id, conn.host_id);
                return Ok(());
            }
        }

        Err(format!("Task {} not found on any host", task_id))
    }

    /// Send input to a running task
    #[allow(dead_code)]
    pub async fn send_input(&self, task_id: &str, content: String) -> Result<(), String> {
        let connections = self.connections.read().await;

        for conn in connections.values() {
            if conn.active_tasks.contains(&task_id.to_string()) {
                conn.tx
                    .send(ServerToGatewayMessage::TaskInput {
                        task_id: task_id.to_string(),
                        content,
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                debug!("Sent input for task {} to host {}", task_id, conn.host_id);
                return Ok(());
            }
        }

        Err(format!("Task {} not found on any host", task_id))
    }

    /// Request available models from a specific gateway host
    pub async fn request_models(&self, host_id: &str) -> Result<Vec<ProviderInfo>, String> {
        let connections = self.connections.read().await;
        
        let conn = connections
            .get(host_id)
            .ok_or_else(|| format!("Host {} not found", host_id))?;

        // Create a request ID and oneshot channel for the response
        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        // Store the pending request
        {
            let mut pending = self.pending_model_requests.write().await;
            pending.insert(request_id.clone(), tx);
        }

        // Send the request to the gateway
        conn.tx
            .send(ServerToGatewayMessage::ModelsRequest { request_id: request_id.clone() })
            .await
            .map_err(|e| {
                // Clean up pending request on send failure
                let pending = self.pending_model_requests.clone();
                let req_id = request_id.clone();
                tokio::spawn(async move {
                    pending.write().await.remove(&req_id);
                });
                format!("Failed to send models request: {}", e)
            })?;

        drop(connections); // Release the read lock before waiting

        // Wait for response with timeout
        match tokio::time::timeout(Duration::from_secs(30), rx).await {
            Ok(Ok(providers)) => Ok(providers),
            Ok(Err(_)) => {
                // Channel closed without response
                Err("Models request was cancelled".to_string())
            }
            Err(_) => {
                // Timeout - clean up pending request
                self.pending_model_requests.write().await.remove(&request_id);
                Err("Models request timed out".to_string())
            }
        }
    }

    /// Handle models response from gateway
    pub async fn handle_models_response(&self, request_id: &str, providers: Vec<ProviderInfo>) {
        let mut pending = self.pending_model_requests.write().await;
        if let Some(tx) = pending.remove(request_id) {
            let _ = tx.send(providers);
            debug!("Delivered models response for request {}", request_id);
        } else {
            warn!("Received models response for unknown request {}", request_id);
        }
    }

    /// List all connected hosts
    pub async fn list_hosts(&self) -> Vec<HostStatus> {
        let connections = self.connections.read().await;
        let now = Instant::now();

        connections
            .values()
            .map(|conn| {
                let status = if conn.active_tasks.is_empty() {
                    HostConnectionStatus::Online
                } else {
                    HostConnectionStatus::Busy
                };

                HostStatus {
                    host_id: conn.host_id.clone(),
                    name: conn.capabilities.name.clone(),
                    status,
                    capabilities: conn.capabilities.clone(),
                    active_tasks: conn.active_tasks.clone(),
                    last_heartbeat: now.duration_since(conn.last_heartbeat).as_secs(),
                    connected_at: now.duration_since(conn.connected_at).as_secs(),
                }
            })
            .collect()
    }

    /// Clean up stale connections (heartbeat timeout)
    pub async fn cleanup_stale_connections(&self, timeout: Duration) {
        let mut connections = self.connections.write().await;
        let now = Instant::now();

        connections.retain(|host_id, conn| {
            if now.duration_since(conn.last_heartbeat) > timeout {
                warn!("Host {} heartbeat timeout, removing", host_id);
                false
            } else {
                true
            }
        });
    }

    /// Get the number of connected hosts
    #[allow(dead_code)]
    pub async fn host_count(&self) -> usize {
        self.connections.read().await.len()
    }
}

impl Default for GatewayManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_test_capabilities() -> HostCapabilities {
        HostCapabilities {
            name: "Test Host".to_string(),
            agents: vec!["opencode".to_string()],
            max_concurrent: 2,
            cwd: "/home/user".to_string(),
            labels: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_register_and_list_hosts() {
        let manager = GatewayManager::new();
        let (tx, _rx) = mpsc::channel(10);

        manager
            .register_host("host-1".to_string(), create_test_capabilities(), tx)
            .await;

        let hosts = manager.list_hosts().await;
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].host_id, "host-1");
        assert_eq!(hosts[0].name, "Test Host");
        assert_eq!(hosts[0].status, HostConnectionStatus::Online);
    }

    #[tokio::test]
    async fn test_unregister_host() {
        let manager = GatewayManager::new();
        let (tx, _rx) = mpsc::channel(10);

        manager
            .register_host("host-1".to_string(), create_test_capabilities(), tx)
            .await;
        assert_eq!(manager.host_count().await, 1);

        manager.unregister_host("host-1").await;
        assert_eq!(manager.host_count().await, 0);
    }

    #[tokio::test]
    async fn test_register_replaces_existing() {
        let manager = GatewayManager::new();
        let (tx1, _rx1) = mpsc::channel(10);
        let (tx2, _rx2) = mpsc::channel(10);

        manager
            .register_host("host-1".to_string(), create_test_capabilities(), tx1)
            .await;

        // Register same host again
        let mut caps2 = create_test_capabilities();
        caps2.name = "Updated Host".to_string();
        manager
            .register_host("host-1".to_string(), caps2, tx2)
            .await;

        let hosts = manager.list_hosts().await;
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].name, "Updated Host");
    }

    #[tokio::test]
    async fn test_dispatch_task_success() {
        let manager = GatewayManager::new();
        let (tx, mut rx) = mpsc::channel(10);

        manager
            .register_host("host-1".to_string(), create_test_capabilities(), tx)
            .await;

        let task = GatewayTaskRequest {
            task_id: "task-1".to_string(),
            prompt: "test prompt".to_string(),
            cwd: "/tmp".to_string(),
            agent_type: "opencode".to_string(),
            model: None,
            env: HashMap::new(),
            timeout: None,
            metadata: serde_json::Value::Null,
        };

        let result = manager.dispatch_task(task).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "host-1");

        // Should have received the task
        let msg = rx.recv().await;
        assert!(matches!(msg, Some(ServerToGatewayMessage::TaskExecute { .. })));
    }

    #[tokio::test]
    async fn test_dispatch_task_no_available_host() {
        let manager = GatewayManager::new();

        let task = GatewayTaskRequest {
            task_id: "task-1".to_string(),
            prompt: "test prompt".to_string(),
            cwd: "/tmp".to_string(),
            agent_type: "opencode".to_string(),
            model: None,
            env: HashMap::new(),
            timeout: None,
            metadata: serde_json::Value::Null,
        };

        let result = manager.dispatch_task(task).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No available host"));
    }

    #[tokio::test]
    async fn test_dispatch_task_wrong_agent_type() {
        let manager = GatewayManager::new();
        let (tx, _rx) = mpsc::channel(10);

        manager
            .register_host("host-1".to_string(), create_test_capabilities(), tx)
            .await;

        let task = GatewayTaskRequest {
            task_id: "task-1".to_string(),
            prompt: "test prompt".to_string(),
            cwd: "/tmp".to_string(),
            agent_type: "claude-code".to_string(), // Not supported by test host
            model: None,
            env: HashMap::new(),
            timeout: None,
            metadata: serde_json::Value::Null,
        };

        let result = manager.dispatch_task(task).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_task_completed_removes_from_active() {
        let manager = GatewayManager::new();
        let (tx, _rx) = mpsc::channel(10);

        manager
            .register_host("host-1".to_string(), create_test_capabilities(), tx)
            .await;

        let task = GatewayTaskRequest {
            task_id: "task-1".to_string(),
            prompt: "test".to_string(),
            cwd: "/tmp".to_string(),
            agent_type: "opencode".to_string(),
            model: None,
            env: HashMap::new(),
            timeout: None,
            metadata: serde_json::Value::Null,
        };

        let _ = manager.dispatch_task(task).await;

        // Host should now be busy
        let hosts = manager.list_hosts().await;
        assert_eq!(hosts[0].active_tasks.len(), 1);

        // Complete the task
        manager
            .handle_task_completed(
                "host-1",
                "task-1",
                TaskResult {
                    success: true,
                    exit_code: Some(0),
                    output: None,
                    duration: Some(100),
                    files_changed: vec![],
                },
            )
            .await;

        // Host should be online again
        let hosts = manager.list_hosts().await;
        assert_eq!(hosts[0].active_tasks.len(), 0);
        assert_eq!(hosts[0].status, HostConnectionStatus::Online);
    }

    #[tokio::test]
    async fn test_heartbeat_update() {
        let manager = GatewayManager::new();
        let (tx, _rx) = mpsc::channel(10);

        manager
            .register_host("host-1".to_string(), create_test_capabilities(), tx)
            .await;

        // Wait a bit
        tokio::time::sleep(Duration::from_millis(100)).await;

        let hosts1 = manager.list_hosts().await;
        let last_hb1 = hosts1[0].last_heartbeat;

        // Update heartbeat
        manager.update_heartbeat("host-1").await;

        let hosts2 = manager.list_hosts().await;
        let last_hb2 = hosts2[0].last_heartbeat;

        // last_heartbeat should be smaller (more recent) after update
        assert!(last_hb2 <= last_hb1);
    }

    #[tokio::test]
    async fn test_cleanup_stale_connections() {
        let manager = GatewayManager::new();
        let (tx, _rx) = mpsc::channel(10);

        manager
            .register_host("host-1".to_string(), create_test_capabilities(), tx)
            .await;

        assert_eq!(manager.host_count().await, 1);

        // Cleanup with very short timeout should remove the host
        manager
            .cleanup_stale_connections(Duration::from_nanos(1))
            .await;

        // Wait a tiny bit for cleanup to complete
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert_eq!(manager.host_count().await, 0);
    }

    #[tokio::test]
    async fn test_event_broadcast() {
        let manager = GatewayManager::new();
        let mut receiver = manager.subscribe();
        let (tx, _rx) = mpsc::channel(10);

        manager
            .register_host("host-1".to_string(), create_test_capabilities(), tx)
            .await;

        let event = GatewayAgentEvent {
            event_type: GatewayAgentEventType::Log,
            content: Some("Test log".to_string()),
            data: serde_json::Value::Null,
            timestamp: 12345,
        };

        manager.handle_task_event("host-1", "task-1", event).await;

        // Should receive the broadcast
        let received = receiver.try_recv();
        assert!(received.is_ok());
        let broadcast = received.unwrap();
        assert_eq!(broadcast.task_id, "task-1");
        assert_eq!(broadcast.host_id, "host-1");
    }
}
