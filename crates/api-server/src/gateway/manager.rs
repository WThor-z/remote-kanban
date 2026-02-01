//! Gateway Manager - manages connections to remote Agent Gateways

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, error, info, warn};

use super::protocol::*;

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
#[derive(Debug, Clone)]
pub struct BroadcastTaskEvent {
    pub task_id: String,
    pub host_id: String,
    pub event: GatewayAgentEvent,
}

/// Gateway Manager - central hub for gateway connections
pub struct GatewayManager {
    connections: Arc<RwLock<HashMap<String, HostConnection>>>,
    event_tx: broadcast::Sender<BroadcastTaskEvent>,
}

impl GatewayManager {
    /// Create a new Gateway Manager
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        }
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

        info!(
            "Task {} completed on host {}: success={}",
            task_id, host_id, result.success
        );
        // TODO: Update task in database
    }

    /// Handle task failed event from gateway
    pub async fn handle_task_failed(&self, host_id: &str, task_id: &str, error: &str) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(host_id) {
            conn.active_tasks.retain(|id| id != task_id);
        }

        error!("Task {} failed on host {}: {}", task_id, host_id, error);
        // TODO: Update task in database
    }

    /// Abort a running task
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

    #[tokio::test]
    async fn test_register_and_list_hosts() {
        let manager = GatewayManager::new();
        let (tx, _rx) = mpsc::channel(10);

        let capabilities = HostCapabilities {
            name: "Test Host".to_string(),
            agents: vec!["opencode".to_string()],
            max_concurrent: 2,
            cwd: "/home/user".to_string(),
            labels: HashMap::new(),
        };

        manager.register_host("host-1".to_string(), capabilities, tx).await;

        let hosts = manager.list_hosts().await;
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].host_id, "host-1");
        assert_eq!(hosts[0].name, "Test Host");
    }

    #[tokio::test]
    async fn test_unregister_host() {
        let manager = GatewayManager::new();
        let (tx, _rx) = mpsc::channel(10);

        let capabilities = HostCapabilities {
            name: "Test Host".to_string(),
            agents: vec!["opencode".to_string()],
            max_concurrent: 2,
            cwd: "/home/user".to_string(),
            labels: HashMap::new(),
        };

        manager.register_host("host-1".to_string(), capabilities, tx).await;
        assert_eq!(manager.host_count().await, 1);

        manager.unregister_host("host-1").await;
        assert_eq!(manager.host_count().await, 0);
    }
}
