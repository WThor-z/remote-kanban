use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

const DEFAULT_HOST_TOKEN_SECRET: &str = "dev-host-token-secret";
const DEFAULT_HOST_TOKEN_TTL_SECONDS: i64 = 60 * 60 * 24 * 30;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HostEnrollmentStatus {
    Active,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostTokenClaims {
    pub sub: String,
    pub org_id: String,
    pub host_id: String,
    pub token_version: u64,
    pub exp: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostSummary {
    pub org_id: String,
    pub host_id: String,
    pub name: Option<String>,
    pub status: HostEnrollmentStatus,
    pub token_version: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub disabled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuedHostToken {
    pub org_id: String,
    pub host_id: String,
    pub token: String,
    pub token_type: String,
    pub expires_at: DateTime<Utc>,
    pub token_version: u64,
    pub status: HostEnrollmentStatus,
}

#[derive(Debug, Error)]
pub enum HostStoreError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("storage error: {0}")]
    Storage(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostRecord {
    org_id: String,
    host_id: String,
    name: Option<String>,
    status: HostEnrollmentStatus,
    token_version: u64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    disabled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostAuditEvent {
    id: Uuid,
    org_id: String,
    host_id: String,
    action: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Default)]
struct HostState {
    hosts: HashMap<String, HostRecord>,
    audit_events: Vec<HostAuditEvent>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct StoredHostState {
    hosts: Vec<HostRecord>,
    audit_events: Vec<HostAuditEvent>,
}

impl From<StoredHostState> for HostState {
    fn from(value: StoredHostState) -> Self {
        Self {
            hosts: value
                .hosts
                .into_iter()
                .map(|record| (host_key(&record.org_id, &record.host_id), record))
                .collect(),
            audit_events: value.audit_events,
        }
    }
}

impl From<&HostState> for StoredHostState {
    fn from(value: &HostState) -> Self {
        Self {
            hosts: value.hosts.values().cloned().collect(),
            audit_events: value.audit_events.clone(),
        }
    }
}

#[derive(Clone)]
pub struct HostStore {
    state: Arc<RwLock<HostState>>,
    file_path: PathBuf,
    jwt_secret: String,
    token_ttl_seconds: i64,
}

impl HostStore {
    pub async fn new(base_dir: PathBuf) -> Result<Self, HostStoreError> {
        tokio::fs::create_dir_all(&base_dir).await.map_err(|err| {
            HostStoreError::Storage(format!("Failed to create host dir: {}", err))
        })?;

        let file_path = base_dir.join("state.json");
        let state = load_state(&file_path).await?;
        let jwt_secret = std::env::var("VK_HOST_JWT_SECRET")
            .unwrap_or_else(|_| DEFAULT_HOST_TOKEN_SECRET.to_string());
        let token_ttl_seconds = std::env::var("VK_HOST_TOKEN_TTL_SECONDS")
            .ok()
            .and_then(|raw| raw.parse::<i64>().ok())
            .filter(|ttl| *ttl > 0)
            .unwrap_or(DEFAULT_HOST_TOKEN_TTL_SECONDS);

        Ok(Self {
            state: Arc::new(RwLock::new(state)),
            file_path,
            jwt_secret,
            token_ttl_seconds,
        })
    }

    pub async fn enroll_host(
        &self,
        org_id: &str,
        host_id: &str,
        name: Option<String>,
    ) -> Result<IssuedHostToken, HostStoreError> {
        let org_id = normalize_org_id(org_id)?;
        let host_id = normalize_host_id(host_id)?;
        let name = normalize_optional_name(name);

        let mut state = self.state.write().await;

        if state
            .hosts
            .values()
            .any(|record| record.host_id == host_id && record.org_id != org_id)
        {
            return Err(HostStoreError::Conflict(format!(
                "Host id '{}' is already enrolled by another organization",
                host_id
            )));
        }

        let now = Utc::now();
        let key = host_key(&org_id, &host_id);
        let record = if let Some(existing) = state.hosts.get_mut(&key) {
            existing.status = HostEnrollmentStatus::Active;
            existing.updated_at = now;
            existing.disabled_at = None;
            existing.token_version = existing.token_version.saturating_add(1);
            if name.is_some() {
                existing.name = name.clone();
            }
            existing.clone()
        } else {
            let record = HostRecord {
                org_id: org_id.clone(),
                host_id: host_id.clone(),
                name,
                status: HostEnrollmentStatus::Active,
                token_version: 1,
                created_at: now,
                updated_at: now,
                disabled_at: None,
            };
            state.hosts.insert(key, record.clone());
            record
        };

        append_audit(&mut state, &record, "host.enrolled");
        let issued = self.issue_token(&record)?;
        persist_state(&self.file_path, &state).await?;
        Ok(issued)
    }

    pub async fn rotate_token(
        &self,
        org_id: &str,
        host_id: &str,
    ) -> Result<IssuedHostToken, HostStoreError> {
        let org_id = normalize_org_id(org_id)?;
        let host_id = normalize_host_id(host_id)?;
        let key = host_key(&org_id, &host_id);

        let mut state = self.state.write().await;
        let record = state
            .hosts
            .get_mut(&key)
            .ok_or_else(|| HostStoreError::NotFound("Host enrollment not found".to_string()))?;

        if record.status != HostEnrollmentStatus::Active {
            return Err(HostStoreError::Forbidden(format!(
                "Host '{}' is disabled",
                record.host_id
            )));
        }

        record.token_version = record.token_version.saturating_add(1);
        record.updated_at = Utc::now();
        let record = record.clone();

        append_audit(&mut state, &record, "host.token_rotated");
        let issued = self.issue_token(&record)?;
        persist_state(&self.file_path, &state).await?;
        Ok(issued)
    }

    pub async fn disable_host(
        &self,
        org_id: &str,
        host_id: &str,
    ) -> Result<HostSummary, HostStoreError> {
        let org_id = normalize_org_id(org_id)?;
        let host_id = normalize_host_id(host_id)?;
        let key = host_key(&org_id, &host_id);

        let mut state = self.state.write().await;
        let record = state
            .hosts
            .get_mut(&key)
            .ok_or_else(|| HostStoreError::NotFound("Host enrollment not found".to_string()))?;

        let now = Utc::now();
        record.status = HostEnrollmentStatus::Disabled;
        record.disabled_at = Some(now);
        record.updated_at = now;
        let record = record.clone();

        append_audit(&mut state, &record, "host.disabled");
        persist_state(&self.file_path, &state).await?;
        Ok(to_summary(&record))
    }

    pub async fn list_hosts(&self, org_id: &str) -> Result<Vec<HostSummary>, HostStoreError> {
        let org_id = normalize_org_id(org_id)?;
        let state = self.state.read().await;

        let mut hosts = state
            .hosts
            .values()
            .filter(|record| record.org_id == org_id)
            .map(to_summary)
            .collect::<Vec<_>>();
        hosts.sort_by(|left, right| left.host_id.cmp(&right.host_id));
        Ok(hosts)
    }

    pub async fn verify_connection_token(
        &self,
        token: &str,
        expected_host_id: &str,
    ) -> Result<HostSummary, HostStoreError> {
        let claims = self.decode_host_token(token)?;
        let expected_host_id = normalize_host_id(expected_host_id)?;
        if claims.host_id != expected_host_id {
            return Err(HostStoreError::Forbidden(
                "host_id claim does not match query hostId".to_string(),
            ));
        }

        let state = self.state.read().await;
        let key = host_key(&claims.org_id, &claims.host_id);
        let record = state
            .hosts
            .get(&key)
            .ok_or_else(|| HostStoreError::Unauthorized("Host enrollment not found".to_string()))?;

        if record.status != HostEnrollmentStatus::Active {
            return Err(HostStoreError::Forbidden("Host is disabled".to_string()));
        }
        if record.token_version != claims.token_version {
            return Err(HostStoreError::Unauthorized(
                "Token version is stale; rotate/enroll required".to_string(),
            ));
        }

        Ok(to_summary(record))
    }

    pub fn decode_host_token(&self, token: &str) -> Result<HostTokenClaims, HostStoreError> {
        let data = decode::<HostTokenClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        )
        .map_err(|err| HostStoreError::Unauthorized(format!("Invalid host token: {}", err)))?;
        Ok(data.claims)
    }

    fn issue_token(&self, record: &HostRecord) -> Result<IssuedHostToken, HostStoreError> {
        let expires_at = Utc::now() + Duration::seconds(self.token_ttl_seconds);
        let exp = usize::try_from(expires_at.timestamp())
            .map_err(|_| HostStoreError::Storage("Failed to encode expiration".to_string()))?;

        let claims = HostTokenClaims {
            sub: format!("host:{}", record.host_id),
            org_id: record.org_id.clone(),
            host_id: record.host_id.clone(),
            token_version: record.token_version,
            exp,
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|err| HostStoreError::Storage(format!("Failed to encode host token: {}", err)))?;

        Ok(IssuedHostToken {
            org_id: record.org_id.clone(),
            host_id: record.host_id.clone(),
            token,
            token_type: "Bearer".to_string(),
            expires_at,
            token_version: record.token_version,
            status: record.status,
        })
    }
}

fn to_summary(record: &HostRecord) -> HostSummary {
    HostSummary {
        org_id: record.org_id.clone(),
        host_id: record.host_id.clone(),
        name: record.name.clone(),
        status: record.status,
        token_version: record.token_version,
        created_at: record.created_at,
        updated_at: record.updated_at,
        disabled_at: record.disabled_at,
    }
}

fn host_key(org_id: &str, host_id: &str) -> String {
    format!("{}:{}", org_id, host_id)
}

fn append_audit(state: &mut HostState, record: &HostRecord, action: &str) {
    state.audit_events.push(HostAuditEvent {
        id: Uuid::new_v4(),
        org_id: record.org_id.clone(),
        host_id: record.host_id.clone(),
        action: action.to_string(),
        created_at: Utc::now(),
    });
}

fn normalize_org_id(value: &str) -> Result<String, HostStoreError> {
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        return Err(HostStoreError::InvalidInput(
            "org_id cannot be empty".to_string(),
        ));
    }
    if normalized.len() > 128 {
        return Err(HostStoreError::InvalidInput(
            "org_id is too long".to_string(),
        ));
    }
    Ok(normalized)
}

fn normalize_host_id(value: &str) -> Result<String, HostStoreError> {
    let normalized = value.trim().to_lowercase();
    if normalized.is_empty() {
        return Err(HostStoreError::InvalidInput(
            "host_id cannot be empty".to_string(),
        ));
    }
    if !normalized
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(HostStoreError::InvalidInput(
            "host_id supports only [a-zA-Z0-9-_]".to_string(),
        ));
    }
    Ok(normalized)
}

fn normalize_optional_name(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

async fn load_state(path: &Path) -> Result<HostState, HostStoreError> {
    if !path.exists() {
        return Ok(HostState::default());
    }
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|err| HostStoreError::Storage(format!("Failed to read host state: {}", err)))?;
    if content.trim().is_empty() {
        return Ok(HostState::default());
    }
    let state: StoredHostState = serde_json::from_str(&content)
        .map_err(|err| HostStoreError::Storage(format!("Failed to parse host state: {}", err)))?;
    Ok(state.into())
}

async fn persist_state(path: &Path, state: &HostState) -> Result<(), HostStoreError> {
    let content = serde_json::to_string_pretty(&StoredHostState::from(state)).map_err(|err| {
        HostStoreError::Storage(format!("Failed to serialize host state: {}", err))
    })?;

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|err| {
            HostStoreError::Storage(format!("Failed to create host dir: {}", err))
        })?;
    }

    tokio::fs::write(path, content)
        .await
        .map_err(|err| HostStoreError::Storage(format!("Failed to write host state: {}", err)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    async fn build_store() -> (HostStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store = HostStore::new(temp_dir.path().join("hosts")).await.unwrap();
        (store, temp_dir)
    }

    #[tokio::test]
    async fn enroll_rotate_disable_host_token_flow() {
        let (store, _temp_dir) = build_store().await;
        let enrolled = store
            .enroll_host("org-a", "host-alpha", Some("Alpha".to_string()))
            .await
            .unwrap();
        let claims = store.decode_host_token(&enrolled.token).unwrap();
        assert_eq!(claims.org_id, "org-a");
        assert_eq!(claims.host_id, "host-alpha");
        assert_eq!(claims.token_version, 1);

        let verified = store
            .verify_connection_token(&enrolled.token, "host-alpha")
            .await
            .unwrap();
        assert_eq!(verified.status, HostEnrollmentStatus::Active);

        let rotated = store.rotate_token("org-a", "host-alpha").await.unwrap();
        assert_eq!(rotated.token_version, 2);

        let old_err = store
            .verify_connection_token(&enrolled.token, "host-alpha")
            .await
            .unwrap_err();
        assert!(matches!(old_err, HostStoreError::Unauthorized(_)));

        let disabled = store.disable_host("org-a", "host-alpha").await.unwrap();
        assert_eq!(disabled.status, HostEnrollmentStatus::Disabled);

        let disabled_err = store
            .verify_connection_token(&rotated.token, "host-alpha")
            .await
            .unwrap_err();
        assert!(matches!(disabled_err, HostStoreError::Forbidden(_)));
    }

    #[tokio::test]
    async fn reject_host_id_reused_by_another_org() {
        let (store, _temp_dir) = build_store().await;
        store
            .enroll_host("org-a", "host-alpha", Some("Alpha".to_string()))
            .await
            .unwrap();

        let err = store
            .enroll_host("org-b", "host-alpha", Some("Other".to_string()))
            .await
            .unwrap_err();
        assert!(matches!(err, HostStoreError::Conflict(_)));
    }

    #[tokio::test]
    async fn reject_token_when_query_host_does_not_match_claim() {
        let (store, _temp_dir) = build_store().await;
        let token = store
            .enroll_host("org-a", "host-alpha", None)
            .await
            .unwrap()
            .token;

        let err = store
            .verify_connection_token(&token, "host-beta")
            .await
            .unwrap_err();
        assert!(matches!(err, HostStoreError::Forbidden(_)));
    }
}
