use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub org_id: String,
    pub actor: String,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default)]
    pub detail: Value,
}

impl AuditEvent {
    pub fn new(
        org_id: impl Into<String>,
        actor: impl Into<String>,
        action: impl Into<String>,
        task_id: Option<Uuid>,
        execution_id: Option<Uuid>,
        host_id: Option<String>,
        status: Option<String>,
        detail: Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            org_id: org_id.into(),
            actor: actor.into(),
            action: action.into(),
            task_id,
            execution_id,
            host_id,
            status,
            detail,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuditListQuery {
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub org_id: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub task_id: Option<Uuid>,
    #[serde(default)]
    pub execution_id: Option<Uuid>,
    #[serde(default)]
    pub host_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditListResponse {
    pub items: Vec<AuditEvent>,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
}
