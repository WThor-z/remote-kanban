//! Gateway protocol types for Agent Gateway communication

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::memory::MemoryItem;

/// Host capabilities - describes what agents a host supports
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostCapabilities {
    pub name: String,
    pub agents: Vec<String>,
    pub max_concurrent: u32,
    pub cwd: String,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

/// Task request sent from server to gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayTaskRequest {
    pub task_id: String,
    pub prompt: String,
    pub cwd: String,
    pub agent_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GatewayMemoryAction {
    #[serde(rename = "settings.get")]
    SettingsGet,
    #[serde(rename = "settings.update")]
    SettingsUpdate,
    #[serde(rename = "items.list")]
    ItemsList,
    #[serde(rename = "items.create")]
    ItemsCreate,
    #[serde(rename = "items.update")]
    ItemsUpdate,
    #[serde(rename = "items.delete")]
    ItemsDelete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GatewayMemorySyncOp {
    Upsert,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayMemorySync {
    pub host_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub op: GatewayMemorySyncOp,
    #[serde(default)]
    pub items: Vec<MemoryItem>,
}

/// Gateway agent event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayAgentEventType {
    Log,
    Thinking,
    ToolCall,
    ToolResult,
    FileChange,
    Message,
    Error,
    Stdout,
    Stderr,
    Output,
    /// Task completed successfully (synthetic event for internal use)
    Completed,
    /// Task failed (synthetic event for internal use)
    Failed,
}

/// Gateway agent event - emitted during task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayAgentEvent {
    #[serde(rename = "type")]
    pub event_type: GatewayAgentEventType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default)]
    pub data: serde_json::Value,
    pub timestamp: u64,
}

/// Task result - returned when task completes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResult {
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
    #[serde(default)]
    pub files_changed: Vec<String>,
}

/// Gateway -> Server messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum GatewayToServerMessage {
    #[serde(rename = "register")]
    Register {
        #[serde(rename = "hostId")]
        host_id: String,
        capabilities: HostCapabilities,
    },
    #[serde(rename = "heartbeat")]
    Heartbeat { timestamp: u64 },
    #[serde(rename = "task:started")]
    TaskStarted {
        #[serde(rename = "taskId")]
        task_id: String,
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    #[serde(rename = "task:event")]
    TaskEvent {
        #[serde(rename = "taskId")]
        task_id: String,
        event: GatewayAgentEvent,
    },
    #[serde(rename = "task:completed")]
    TaskCompleted {
        #[serde(rename = "taskId")]
        task_id: String,
        result: TaskResult,
    },
    #[serde(rename = "task:failed")]
    TaskFailed {
        #[serde(rename = "taskId")]
        task_id: String,
        error: String,
        #[serde(default)]
        details: serde_json::Value,
    },
    #[serde(rename = "models:response")]
    ModelsResponse {
        #[serde(rename = "requestId")]
        request_id: String,
        providers: Vec<ProviderInfo>,
    },
    #[serde(rename = "memory:response")]
    MemoryResponse {
        #[serde(rename = "requestId")]
        request_id: String,
        ok: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    #[serde(rename = "memory:sync")]
    MemorySync { sync: GatewayMemorySync },
}

/// Server -> Gateway messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ServerToGatewayMessage {
    #[serde(rename = "registered")]
    Registered {
        ok: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "task:execute")]
    TaskExecute { task: GatewayTaskRequest },
    #[serde(rename = "task:abort")]
    TaskAbort {
        #[serde(rename = "taskId")]
        task_id: String,
    },
    #[serde(rename = "task:input")]
    TaskInput {
        #[serde(rename = "taskId")]
        task_id: String,
        content: String,
    },
    #[serde(rename = "models:request")]
    ModelsRequest {
        #[serde(rename = "requestId")]
        request_id: String,
    },
    #[serde(rename = "memory:request")]
    MemoryRequest {
        #[serde(rename = "requestId")]
        request_id: String,
        action: GatewayMemoryAction,
        #[serde(default)]
        payload: serde_json::Value,
    },
}

/// Host status - used for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostStatus {
    pub org_id: String,
    pub host_id: String,
    pub name: String,
    pub status: HostConnectionStatus,
    pub capabilities: HostCapabilities,
    pub active_tasks: Vec<String>,
    pub last_heartbeat: u64,
    pub connected_at: u64,
}

/// Host connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostConnectionStatus {
    Online,
    Offline,
    Busy,
}

/// Model information from OpenCode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<ModelCapabilities>,
}

/// Model capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCapabilities {
    pub temperature: bool,
    pub reasoning: bool,
    pub attachment: bool,
    pub toolcall: bool,
}

/// Provider information from OpenCode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub models: Vec<ModelInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_to_server_message_serialization() {
        let msg = GatewayToServerMessage::Register {
            host_id: "host-1".to_string(),
            capabilities: HostCapabilities {
                name: "Test Host".to_string(),
                agents: vec!["opencode".to_string()],
                max_concurrent: 2,
                cwd: "/home/user".to_string(),
                labels: HashMap::new(),
            },
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"register\""));
        assert!(json.contains("\"hostId\":\"host-1\""));
    }

    #[test]
    fn test_server_to_gateway_message_serialization() {
        let msg = ServerToGatewayMessage::Registered {
            ok: true,
            error: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"registered\""));
        assert!(json.contains("\"ok\":true"));
    }

    #[test]
    fn test_gateway_to_server_message_deserialization() {
        let json = r#"{"type":"heartbeat","timestamp":1234567890}"#;
        let msg: GatewayToServerMessage = serde_json::from_str(json).unwrap();

        match msg {
            GatewayToServerMessage::Heartbeat { timestamp } => {
                assert_eq!(timestamp, 1234567890);
            }
            _ => panic!("Expected Heartbeat message"),
        }
    }
}
