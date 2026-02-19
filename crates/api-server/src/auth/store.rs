use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

const DEFAULT_JWT_SECRET: &str = "dev-jwt-secret-change-me";
const DEFAULT_TOKEN_TTL_SECONDS: i64 = 60 * 60 * 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrgRole {
    Owner,
    Admin,
    Member,
    Viewer,
}

impl OrgRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Member => "member",
            Self::Viewer => "viewer",
        }
    }

    pub fn can_manage_members(self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }

    pub fn can_manage_api_keys(self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }
}

impl FromStr for OrgRole {
    type Err = AuthError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "owner" => Ok(Self::Owner),
            "admin" => Ok(Self::Admin),
            "member" => Ok(Self::Member),
            "viewer" => Ok(Self::Viewer),
            _ => Err(AuthError::InvalidInput(format!(
                "Unsupported role '{}'",
                value
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthClaims {
    pub sub: String,
    pub org_id: String,
    pub role: String,
    pub exp: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSummary {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationSummary {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberRecord {
    pub id: Uuid,
    pub user: UserSummary,
    pub org_id: Uuid,
    pub role: OrgRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeySummary {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedApiKey {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub token: String,
    pub key_prefix: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AuthSession {
    pub claims: AuthClaims,
    pub user: UserSummary,
    pub org: OrganizationSummary,
    pub membership: MemberRecord,
}

#[derive(Debug, Error)]
pub enum AuthError {
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
struct Organization {
    id: Uuid,
    name: String,
    slug: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: Uuid,
    email: String,
    display_name: Option<String>,
    password_hash: String,
    created_at: DateTime<Utc>,
    disabled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Membership {
    id: Uuid,
    user_id: Uuid,
    org_id: Uuid,
    role: OrgRole,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiKey {
    id: Uuid,
    org_id: Uuid,
    name: String,
    key_prefix: String,
    token_hash: String,
    created_by: Uuid,
    created_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditEvent {
    id: Uuid,
    org_id: Uuid,
    actor_user_id: Option<Uuid>,
    action: String,
    target_type: String,
    target_id: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Default)]
struct AuthState {
    organizations: HashMap<Uuid, Organization>,
    users: HashMap<Uuid, User>,
    memberships: HashMap<Uuid, Membership>,
    api_keys: HashMap<Uuid, ApiKey>,
    audit_events: Vec<AuditEvent>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct StoredAuthState {
    organizations: Vec<Organization>,
    users: Vec<User>,
    memberships: Vec<Membership>,
    api_keys: Vec<ApiKey>,
    audit_events: Vec<AuditEvent>,
}

impl From<StoredAuthState> for AuthState {
    fn from(value: StoredAuthState) -> Self {
        Self {
            organizations: value
                .organizations
                .into_iter()
                .map(|item| (item.id, item))
                .collect(),
            users: value
                .users
                .into_iter()
                .map(|item| (item.id, item))
                .collect(),
            memberships: value
                .memberships
                .into_iter()
                .map(|item| (item.id, item))
                .collect(),
            api_keys: value
                .api_keys
                .into_iter()
                .map(|item| (item.id, item))
                .collect(),
            audit_events: value.audit_events,
        }
    }
}

impl From<&AuthState> for StoredAuthState {
    fn from(value: &AuthState) -> Self {
        Self {
            organizations: value.organizations.values().cloned().collect(),
            users: value.users.values().cloned().collect(),
            memberships: value.memberships.values().cloned().collect(),
            api_keys: value.api_keys.values().cloned().collect(),
            audit_events: value.audit_events.clone(),
        }
    }
}

#[derive(Clone)]
pub struct AuthStore {
    state: Arc<RwLock<AuthState>>,
    file_path: PathBuf,
    jwt_secret: String,
    token_ttl_seconds: i64,
}

impl AuthStore {
    pub async fn new(base_dir: PathBuf) -> Result<Self, AuthError> {
        tokio::fs::create_dir_all(&base_dir).await.map_err(|err| {
            AuthError::Storage(format!("Failed to create auth directory: {}", err))
        })?;

        let file_path = base_dir.join("state.json");
        let state = load_state(&file_path).await?;
        let jwt_secret =
            std::env::var("VK_AUTH_JWT_SECRET").unwrap_or_else(|_| DEFAULT_JWT_SECRET.to_string());
        let token_ttl_seconds = std::env::var("VK_AUTH_TOKEN_TTL_SECONDS")
            .ok()
            .and_then(|raw| raw.parse::<i64>().ok())
            .filter(|ttl| *ttl > 0)
            .unwrap_or(DEFAULT_TOKEN_TTL_SECONDS);

        Ok(Self {
            state: Arc::new(RwLock::new(state)),
            file_path,
            jwt_secret,
            token_ttl_seconds,
        })
    }

    pub async fn register_owner(
        &self,
        email: &str,
        password: &str,
        display_name: Option<String>,
        org_name: &str,
        org_slug: Option<String>,
    ) -> Result<AuthSession, AuthError> {
        let normalized_email = normalize_email(email)?;
        validate_password(password)?;

        let org_name = org_name.trim();
        if org_name.is_empty() {
            return Err(AuthError::InvalidInput(
                "Organization name cannot be empty".to_string(),
            ));
        }
        let org_slug = normalize_slug(org_slug.as_deref().unwrap_or(org_name))?;

        let mut state = self.state.write().await;
        if state
            .users
            .values()
            .any(|user| user.email == normalized_email)
        {
            return Err(AuthError::Conflict(format!(
                "User '{}' already exists",
                normalized_email
            )));
        }
        if state.organizations.values().any(|org| org.slug == org_slug) {
            return Err(AuthError::Conflict(format!(
                "Organization slug '{}' already exists",
                org_slug
            )));
        }

        let now = Utc::now();
        let user = User {
            id: Uuid::new_v4(),
            email: normalized_email,
            display_name: sanitize_optional_string(display_name),
            password_hash: hash_password(password),
            created_at: now,
            disabled_at: None,
        };
        let org = Organization {
            id: Uuid::new_v4(),
            name: org_name.to_string(),
            slug: org_slug,
            created_at: now,
            updated_at: now,
        };
        let membership = Membership {
            id: Uuid::new_v4(),
            user_id: user.id,
            org_id: org.id,
            role: OrgRole::Owner,
            created_at: now,
            updated_at: now,
        };

        state.users.insert(user.id, user.clone());
        state.organizations.insert(org.id, org.clone());
        state.memberships.insert(membership.id, membership.clone());
        append_audit(
            &mut state,
            org.id,
            Some(user.id),
            "org.created",
            "organization",
            Some(org.id.to_string()),
        );
        persist_state(&self.file_path, &state).await?;
        drop(state);
        self.build_session(user, org, membership).await
    }

    pub async fn login(
        &self,
        email: &str,
        password: &str,
        org_id: Option<Uuid>,
        org_slug: Option<&str>,
    ) -> Result<AuthSession, AuthError> {
        let normalized_email = normalize_email(email)?;
        let state = self.state.read().await;

        let user = state
            .users
            .values()
            .find(|user| user.email == normalized_email)
            .cloned()
            .ok_or_else(|| AuthError::Unauthorized("Invalid credentials".to_string()))?;
        if user.disabled_at.is_some() || !verify_password(&user.password_hash, password) {
            return Err(AuthError::Unauthorized("Invalid credentials".to_string()));
        }

        let memberships: Vec<Membership> = state
            .memberships
            .values()
            .filter(|membership| membership.user_id == user.id)
            .cloned()
            .collect();
        if memberships.is_empty() {
            return Err(AuthError::Forbidden(
                "User does not belong to any organization".to_string(),
            ));
        }

        let membership = if let Some(org_id) = org_id {
            memberships
                .iter()
                .find(|item| item.org_id == org_id)
                .cloned()
                .ok_or_else(|| AuthError::Forbidden("No access to organization".to_string()))?
        } else if let Some(org_slug) = org_slug {
            let slug = normalize_slug(org_slug)?;
            let org = state
                .organizations
                .values()
                .find(|item| item.slug == slug)
                .ok_or_else(|| AuthError::NotFound("Organization not found".to_string()))?;
            memberships
                .iter()
                .find(|item| item.org_id == org.id)
                .cloned()
                .ok_or_else(|| AuthError::Forbidden("No access to organization".to_string()))?
        } else if memberships.len() == 1 {
            memberships[0].clone()
        } else {
            return Err(AuthError::InvalidInput(
                "Multiple organizations found, provide orgId or orgSlug".to_string(),
            ));
        };

        let org = state
            .organizations
            .get(&membership.org_id)
            .cloned()
            .ok_or_else(|| AuthError::NotFound("Organization not found".to_string()))?;
        drop(state);
        self.build_session(user, org, membership).await
    }

    pub async fn authorize_bearer(&self, token: &str) -> Result<AuthSession, AuthError> {
        let claims = self.decode_claims(token)?;
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AuthError::Unauthorized("Invalid token subject".to_string()))?;
        let org_id = Uuid::parse_str(&claims.org_id)
            .map_err(|_| AuthError::Unauthorized("Invalid token org_id".to_string()))?;
        let token_role = OrgRole::from_str(&claims.role)?;

        let state = self.state.read().await;
        let user = state
            .users
            .get(&user_id)
            .cloned()
            .ok_or_else(|| AuthError::Unauthorized("User not found".to_string()))?;
        let org = state
            .organizations
            .get(&org_id)
            .cloned()
            .ok_or_else(|| AuthError::Unauthorized("Organization not found".to_string()))?;
        let membership = state
            .memberships
            .values()
            .find(|membership| membership.user_id == user_id && membership.org_id == org_id)
            .cloned()
            .ok_or_else(|| AuthError::Unauthorized("Membership not found".to_string()))?;
        if membership.role != token_role {
            return Err(AuthError::Unauthorized(
                "Token role does not match membership".to_string(),
            ));
        }

        Ok(AuthSession {
            claims,
            user: user_to_summary(&user),
            org: organization_to_summary(&org),
            membership: membership_to_record(&membership, &user),
        })
    }

    pub async fn list_orgs_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(OrganizationSummary, OrgRole)>, AuthError> {
        let state = self.state.read().await;
        let mut orgs = Vec::new();
        for membership in state
            .memberships
            .values()
            .filter(|membership| membership.user_id == user_id)
        {
            if let Some(org) = state.organizations.get(&membership.org_id) {
                orgs.push((organization_to_summary(org), membership.role));
            }
        }
        orgs.sort_by(|left, right| left.0.slug.cmp(&right.0.slug));
        Ok(orgs)
    }

    pub async fn create_org_for_user(
        &self,
        user_id: Uuid,
        name: &str,
        slug: Option<String>,
    ) -> Result<OrganizationSummary, AuthError> {
        let normalized_name = name.trim();
        if normalized_name.is_empty() {
            return Err(AuthError::InvalidInput(
                "Organization name cannot be empty".to_string(),
            ));
        }
        let normalized_slug = normalize_slug(slug.as_deref().unwrap_or(normalized_name))?;

        let mut state = self.state.write().await;
        if !state.users.contains_key(&user_id) {
            return Err(AuthError::NotFound("User not found".to_string()));
        }
        if state
            .organizations
            .values()
            .any(|organization| organization.slug == normalized_slug)
        {
            return Err(AuthError::Conflict(format!(
                "Organization slug '{}' already exists",
                normalized_slug
            )));
        }

        let now = Utc::now();
        let organization = Organization {
            id: Uuid::new_v4(),
            name: normalized_name.to_string(),
            slug: normalized_slug,
            created_at: now,
            updated_at: now,
        };
        let membership = Membership {
            id: Uuid::new_v4(),
            user_id,
            org_id: organization.id,
            role: OrgRole::Owner,
            created_at: now,
            updated_at: now,
        };
        state
            .organizations
            .insert(organization.id, organization.clone());
        state.memberships.insert(membership.id, membership);
        append_audit(
            &mut state,
            organization.id,
            Some(user_id),
            "org.created",
            "organization",
            Some(organization.id.to_string()),
        );
        persist_state(&self.file_path, &state).await?;
        Ok(organization_to_summary(&organization))
    }

    pub async fn get_org_for_user(
        &self,
        user_id: Uuid,
        org_id: Uuid,
    ) -> Result<(OrganizationSummary, OrgRole), AuthError> {
        let state = self.state.read().await;
        let org = state
            .organizations
            .get(&org_id)
            .ok_or_else(|| AuthError::NotFound("Organization not found".to_string()))?;
        let membership = ensure_membership(&state, user_id, org_id)?;
        Ok((organization_to_summary(org), membership.role))
    }

    pub async fn list_members(
        &self,
        user_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<MemberRecord>, AuthError> {
        let state = self.state.read().await;
        ensure_membership(&state, user_id, org_id)?;
        let mut members = Vec::new();
        for membership in state
            .memberships
            .values()
            .filter(|membership| membership.org_id == org_id)
        {
            if let Some(user) = state.users.get(&membership.user_id) {
                members.push(membership_to_record(membership, user));
            }
        }
        members.sort_by(|left, right| left.user.email.cmp(&right.user.email));
        Ok(members)
    }

    pub async fn add_member(
        &self,
        actor_user_id: Uuid,
        org_id: Uuid,
        email: &str,
        role: OrgRole,
    ) -> Result<MemberRecord, AuthError> {
        let normalized_email = normalize_email(email)?;
        let mut state = self.state.write().await;
        let actor_membership = ensure_membership(&state, actor_user_id, org_id)?;
        if !actor_membership.role.can_manage_members() {
            return Err(AuthError::Forbidden(
                "Only owner/admin can manage members".to_string(),
            ));
        }

        let user = state
            .users
            .values()
            .find(|user| user.email == normalized_email)
            .cloned()
            .ok_or_else(|| AuthError::NotFound("User not found".to_string()))?;
        let now = Utc::now();

        let existing = state
            .memberships
            .values_mut()
            .find(|membership| membership.user_id == user.id && membership.org_id == org_id);
        let membership = if let Some(existing) = existing {
            existing.role = role;
            existing.updated_at = now;
            existing.clone()
        } else {
            let membership = Membership {
                id: Uuid::new_v4(),
                user_id: user.id,
                org_id,
                role,
                created_at: now,
                updated_at: now,
            };
            state.memberships.insert(membership.id, membership.clone());
            membership
        };
        append_audit(
            &mut state,
            org_id,
            Some(actor_user_id),
            "membership.upserted",
            "membership",
            Some(membership.id.to_string()),
        );
        persist_state(&self.file_path, &state).await?;
        Ok(membership_to_record(&membership, &user))
    }

    pub async fn create_api_key(
        &self,
        actor_user_id: Uuid,
        org_id: Uuid,
        name: &str,
    ) -> Result<CreatedApiKey, AuthError> {
        let normalized_name = name.trim();
        if normalized_name.is_empty() {
            return Err(AuthError::InvalidInput(
                "API key name cannot be empty".to_string(),
            ));
        }

        let mut state = self.state.write().await;
        let actor_membership = ensure_membership(&state, actor_user_id, org_id)?;
        if !actor_membership.role.can_manage_api_keys() {
            return Err(AuthError::Forbidden(
                "Only owner/admin can manage API keys".to_string(),
            ));
        }

        let token = generate_api_key_token();
        let key_prefix = token.chars().take(12).collect::<String>();
        let api_key = ApiKey {
            id: Uuid::new_v4(),
            org_id,
            name: normalized_name.to_string(),
            key_prefix: key_prefix.clone(),
            token_hash: hash_api_key_token(&token),
            created_by: actor_user_id,
            created_at: Utc::now(),
            revoked_at: None,
        };
        state.api_keys.insert(api_key.id, api_key.clone());
        append_audit(
            &mut state,
            org_id,
            Some(actor_user_id),
            "api_key.created",
            "api_key",
            Some(api_key.id.to_string()),
        );
        persist_state(&self.file_path, &state).await?;

        Ok(CreatedApiKey {
            id: api_key.id,
            org_id: api_key.org_id,
            name: api_key.name,
            token,
            key_prefix,
            created_at: api_key.created_at,
        })
    }

    pub async fn list_api_keys(
        &self,
        user_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<ApiKeySummary>, AuthError> {
        let state = self.state.read().await;
        ensure_membership(&state, user_id, org_id)?;
        let mut keys = state
            .api_keys
            .values()
            .filter(|api_key| api_key.org_id == org_id)
            .map(api_key_to_summary)
            .collect::<Vec<_>>();
        keys.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(keys)
    }

    async fn build_session(
        &self,
        user: User,
        organization: Organization,
        membership: Membership,
    ) -> Result<AuthSession, AuthError> {
        let claims = self.issue_claims(user.id, organization.id, membership.role)?;
        Ok(AuthSession {
            claims,
            user: user_to_summary(&user),
            org: organization_to_summary(&organization),
            membership: membership_to_record(&membership, &user),
        })
    }

    fn issue_claims(
        &self,
        user_id: Uuid,
        org_id: Uuid,
        role: OrgRole,
    ) -> Result<AuthClaims, AuthError> {
        let exp = (Utc::now() + Duration::seconds(self.token_ttl_seconds)).timestamp();
        let exp = usize::try_from(exp)
            .map_err(|_| AuthError::Storage("Failed to encode token expiration".to_string()))?;

        Ok(AuthClaims {
            sub: user_id.to_string(),
            org_id: org_id.to_string(),
            role: role.as_str().to_string(),
            exp,
        })
    }

    pub fn encode_claims(&self, claims: &AuthClaims) -> Result<String, AuthError> {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|err| AuthError::Storage(format!("Failed to encode JWT: {}", err)))
    }

    pub fn decode_claims(&self, token: &str) -> Result<AuthClaims, AuthError> {
        let decoded = decode::<AuthClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        )
        .map_err(|err| AuthError::Unauthorized(format!("Invalid token: {}", err)))?;
        Ok(decoded.claims)
    }
}

fn user_to_summary(user: &User) -> UserSummary {
    UserSummary {
        id: user.id,
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        created_at: user.created_at,
    }
}

fn organization_to_summary(organization: &Organization) -> OrganizationSummary {
    OrganizationSummary {
        id: organization.id,
        name: organization.name.clone(),
        slug: organization.slug.clone(),
        created_at: organization.created_at,
        updated_at: organization.updated_at,
    }
}

fn membership_to_record(membership: &Membership, user: &User) -> MemberRecord {
    MemberRecord {
        id: membership.id,
        user: user_to_summary(user),
        org_id: membership.org_id,
        role: membership.role,
        created_at: membership.created_at,
        updated_at: membership.updated_at,
    }
}

fn api_key_to_summary(api_key: &ApiKey) -> ApiKeySummary {
    ApiKeySummary {
        id: api_key.id,
        org_id: api_key.org_id,
        name: api_key.name.clone(),
        key_prefix: api_key.key_prefix.clone(),
        created_by: api_key.created_by,
        created_at: api_key.created_at,
        revoked_at: api_key.revoked_at,
    }
}

fn ensure_membership(
    state: &AuthState,
    user_id: Uuid,
    org_id: Uuid,
) -> Result<Membership, AuthError> {
    if !state.organizations.contains_key(&org_id) {
        return Err(AuthError::NotFound("Organization not found".to_string()));
    }
    state
        .memberships
        .values()
        .find(|membership| membership.user_id == user_id && membership.org_id == org_id)
        .cloned()
        .ok_or_else(|| AuthError::Forbidden("No access to organization".to_string()))
}

fn append_audit(
    state: &mut AuthState,
    org_id: Uuid,
    actor_user_id: Option<Uuid>,
    action: &str,
    target_type: &str,
    target_id: Option<String>,
) {
    state.audit_events.push(AuditEvent {
        id: Uuid::new_v4(),
        org_id,
        actor_user_id,
        action: action.to_string(),
        target_type: target_type.to_string(),
        target_id,
        created_at: Utc::now(),
    });
}

async fn load_state(path: &Path) -> Result<AuthState, AuthError> {
    if !path.exists() {
        return Ok(AuthState::default());
    }
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|err| AuthError::Storage(format!("Failed to read auth state: {}", err)))?;
    if content.trim().is_empty() {
        return Ok(AuthState::default());
    }
    let stored: StoredAuthState = serde_json::from_str(&content)
        .map_err(|err| AuthError::Storage(format!("Failed to parse auth state: {}", err)))?;
    Ok(stored.into())
}

async fn persist_state(path: &Path, state: &AuthState) -> Result<(), AuthError> {
    let content = serde_json::to_string_pretty(&StoredAuthState::from(state))
        .map_err(|err| AuthError::Storage(format!("Failed to serialize auth state: {}", err)))?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|err| {
            AuthError::Storage(format!("Failed to create auth parent dir: {}", err))
        })?;
    }
    tokio::fs::write(path, content)
        .await
        .map_err(|err| AuthError::Storage(format!("Failed to write auth state: {}", err)))?;
    Ok(())
}

fn sanitize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_email(email: &str) -> Result<String, AuthError> {
    let normalized = email.trim().to_lowercase();
    if normalized.is_empty() || !normalized.contains('@') {
        return Err(AuthError::InvalidInput("Invalid email".to_string()));
    }
    Ok(normalized)
}

fn normalize_slug(value: &str) -> Result<String, AuthError> {
    let mut slug = String::with_capacity(value.len());
    let mut last_was_dash = false;
    for ch in value.chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        return Err(AuthError::InvalidInput(
            "Organization slug cannot be empty".to_string(),
        ));
    }
    Ok(slug)
}

fn validate_password(password: &str) -> Result<(), AuthError> {
    if password.len() < 8 {
        return Err(AuthError::InvalidInput(
            "Password must be at least 8 characters".to_string(),
        ));
    }
    Ok(())
}

fn hash_password(password: &str) -> String {
    let mut salt = [0_u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);

    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update(password.as_bytes());
    let digest = hasher.finalize();

    format!(
        "v1${}${}",
        URL_SAFE_NO_PAD.encode(salt),
        URL_SAFE_NO_PAD.encode(digest)
    )
}

fn verify_password(stored_hash: &str, password: &str) -> bool {
    let mut parts = stored_hash.split('$');
    let version = parts.next();
    let encoded_salt = parts.next();
    let encoded_digest = parts.next();
    if version != Some("v1") || encoded_salt.is_none() || encoded_digest.is_none() {
        return false;
    }

    let salt = match URL_SAFE_NO_PAD.decode(encoded_salt.unwrap()) {
        Ok(value) => value,
        Err(_) => return false,
    };
    let expected_digest = match URL_SAFE_NO_PAD.decode(encoded_digest.unwrap()) {
        Ok(value) => value,
        Err(_) => return false,
    };

    let mut hasher = Sha256::new();
    hasher.update(&salt);
    hasher.update(password.as_bytes());
    let actual_digest = hasher.finalize();
    expected_digest == actual_digest.as_slice()
}

fn generate_api_key_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("vk_{}", URL_SAFE_NO_PAD.encode(bytes))
}

fn hash_api_key_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    async fn build_store() -> (AuthStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store = AuthStore::new(temp_dir.path().join("auth")).await.unwrap();
        (store, temp_dir)
    }

    #[tokio::test]
    async fn register_and_login_roundtrip() {
        let (store, _temp_dir) = build_store().await;
        let session = store
            .register_owner(
                "owner@example.com",
                "verysecurepw",
                Some("Owner".to_string()),
                "Acme",
                None,
            )
            .await
            .unwrap();
        let token = store.encode_claims(&session.claims).unwrap();
        let authed = store.authorize_bearer(&token).await.unwrap();
        assert_eq!(authed.user.email, "owner@example.com");
        assert_eq!(authed.org.id, session.org.id);
    }
}
