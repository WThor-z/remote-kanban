use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{issue_host_jwt, issue_user_jwt, resolve_user_identity},
    state::AppState,
};

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

type RouteError = (StatusCode, Json<ErrorResponse>);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterRequest {
    email: String,
    password: String,
    name: String,
    #[serde(default)]
    org_id: Option<String>,
    #[serde(default)]
    org_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginRequest {
    email: String,
    password: String,
    #[serde(default)]
    org_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateOrgRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HostEnrollRequest {
    host_id: String,
    #[serde(default)]
    expires_in_hours: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HostDisableRequest {
    host_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthResponse {
    token: String,
    expires_at: String,
    user_id: String,
    email: String,
    name: String,
    org_id: String,
    role: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MeResponse {
    user_id: String,
    email: String,
    name: String,
    org_id: String,
    role: String,
    organizations: Vec<OrgSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrgSummary {
    id: String,
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HostTokenResponse {
    host_id: String,
    org_id: String,
    token: String,
    token_version: u32,
    enabled: bool,
    expires_at: String,
}

fn route_error(status: StatusCode, error: impl Into<String>) -> RouteError {
    (
        status,
        Json(ErrorResponse {
            error: error.into(),
        }),
    )
}

fn unauthorized(error: impl Into<String>) -> RouteError {
    route_error(StatusCode::UNAUTHORIZED, error)
}

fn forbidden(error: impl Into<String>) -> RouteError {
    route_error(StatusCode::FORBIDDEN, error)
}

fn bad_request(error: impl Into<String>) -> RouteError {
    route_error(StatusCode::BAD_REQUEST, error)
}

fn conflict(error: impl Into<String>) -> RouteError {
    route_error(StatusCode::CONFLICT, error)
}

fn internal_error(error: impl std::fmt::Display) -> RouteError {
    route_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn default_org_id() -> String {
    std::env::var("VK_DEFAULT_ORG_ID")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "default-org".to_string())
}

fn default_org_name() -> String {
    std::env::var("VK_DEFAULT_ORG_NAME")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Default Organization".to_string())
}

fn format_expiry(exp: usize) -> String {
    DateTime::<Utc>::from_timestamp(exp as i64, 0)
        .map(|value| value.to_rfc3339())
        .unwrap_or_else(|| Utc::now().to_rfc3339())
}

fn is_org_admin(role: &str) -> bool {
    matches!(role, "owner" | "admin")
}

async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), RouteError> {
    let org = if let Some(org_id) = req.org_id.as_deref() {
        let fallback_name = default_org_name();
        let org_name = req.org_name.as_deref().unwrap_or(fallback_name.as_str());
        state
            .auth_store()
            .ensure_organization(org_id, org_name)
            .await
            .map_err(conflict)?
    } else if let Some(org_name) = req.org_name.as_deref() {
        state
            .auth_store()
            .create_organization(org_name)
            .await
            .map_err(conflict)?
    } else {
        state
            .auth_store()
            .ensure_organization(&default_org_id(), &default_org_name())
            .await
            .map_err(conflict)?
    };

    let user = state
        .auth_store()
        .create_user(&req.email, &req.password, &req.name)
        .await
        .map_err(conflict)?;

    let membership = state
        .auth_store()
        .ensure_membership(&user.id, &org.id, "owner")
        .await
        .map_err(conflict)?;

    let (token, exp) =
        issue_user_jwt(&user.id, &org.id, &membership.role, 24).map_err(internal_error)?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            expires_at: format_expiry(exp),
            user_id: user.id,
            email: user.email,
            name: user.name,
            org_id: org.id,
            role: membership.role,
        }),
    ))
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, RouteError> {
    let user = state
        .auth_store()
        .authenticate_user(&req.email, &req.password)
        .await
        .ok_or_else(|| unauthorized("Invalid email or password"))?;

    let mut memberships = state.auth_store().list_memberships_for_user(&user.id).await;
    if memberships.is_empty() {
        let org = state
            .auth_store()
            .ensure_organization(&default_org_id(), &default_org_name())
            .await
            .map_err(internal_error)?;
        let membership = state
            .auth_store()
            .ensure_membership(&user.id, &org.id, "member")
            .await
            .map_err(internal_error)?;
        memberships.push(membership);
    }

    let membership = if let Some(org_id) = req.org_id.as_deref() {
        memberships
            .into_iter()
            .find(|membership| membership.org_id == org_id)
            .ok_or_else(|| forbidden(format!("No membership in organization {}", org_id)))?
    } else {
        memberships
            .into_iter()
            .next()
            .ok_or_else(|| forbidden("No organization membership found"))?
    };

    let (token, exp) = issue_user_jwt(&user.id, &membership.org_id, &membership.role, 24)
        .map_err(internal_error)?;

    Ok(Json(AuthResponse {
        token,
        expires_at: format_expiry(exp),
        user_id: user.id,
        email: user.email,
        name: user.name,
        org_id: membership.org_id,
        role: membership.role,
    }))
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, RouteError> {
    let identity = resolve_user_identity(&headers, true).map_err(unauthorized)?;

    let identity = identity.ok_or_else(|| unauthorized("Missing identity"))?;
    let user = state
        .auth_store()
        .get_user_by_id(&identity.sub)
        .await
        .ok_or_else(|| unauthorized("User not found"))?;

    let organizations = state
        .auth_store()
        .list_organizations_for_user(&user.id)
        .await
        .into_iter()
        .map(|org| OrgSummary {
            id: org.id,
            name: org.name,
        })
        .collect::<Vec<_>>();

    Ok(Json(MeResponse {
        user_id: user.id,
        email: user.email,
        name: user.name,
        org_id: identity.org_id,
        role: identity.role,
        organizations,
    }))
}

async fn list_orgs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<OrgSummary>>, RouteError> {
    let identity = resolve_user_identity(&headers, true).map_err(unauthorized)?;
    let identity = identity.ok_or_else(|| unauthorized("Missing identity"))?;

    let organizations = state
        .auth_store()
        .list_organizations_for_user(&identity.sub)
        .await
        .into_iter()
        .map(|org| OrgSummary {
            id: org.id,
            name: org.name,
        })
        .collect::<Vec<_>>();

    Ok(Json(organizations))
}

async fn create_org(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateOrgRequest>,
) -> Result<(StatusCode, Json<OrgSummary>), RouteError> {
    let identity = resolve_user_identity(&headers, true).map_err(unauthorized)?;
    let identity = identity.ok_or_else(|| unauthorized("Missing identity"))?;

    let org = state
        .auth_store()
        .create_organization(&req.name)
        .await
        .map_err(conflict)?;

    state
        .auth_store()
        .ensure_membership(&identity.sub, &org.id, "owner")
        .await
        .map_err(conflict)?;

    Ok((
        StatusCode::CREATED,
        Json(OrgSummary {
            id: org.id,
            name: org.name,
        }),
    ))
}

async fn enroll_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(org_id): Path<String>,
    Json(req): Json<HostEnrollRequest>,
) -> Result<Json<HostTokenResponse>, RouteError> {
    let identity = resolve_user_identity(&headers, true).map_err(unauthorized)?;
    let identity = identity.ok_or_else(|| unauthorized("Missing identity"))?;
    if identity.org_id != org_id {
        return Err(forbidden("Cannot enroll hosts for another organization"));
    }
    if !is_org_admin(&identity.role) {
        return Err(forbidden("Insufficient role for host enrollment"));
    }

    let enrollment = state
        .auth_store()
        .upsert_host_enrollment(&org_id, &req.host_id)
        .await
        .map_err(conflict)?;

    let expires_in_hours = req.expires_in_hours.unwrap_or(24 * 30).clamp(1, 24 * 365);
    let (token, exp) = issue_host_jwt(
        &enrollment.org_id,
        &enrollment.host_id,
        enrollment.token_version,
        expires_in_hours,
    )
    .map_err(internal_error)?;

    Ok(Json(HostTokenResponse {
        host_id: enrollment.host_id,
        org_id: enrollment.org_id,
        token,
        token_version: enrollment.token_version,
        enabled: enrollment.enabled,
        expires_at: format_expiry(exp),
    }))
}

async fn rotate_host_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(org_id): Path<String>,
    Json(req): Json<HostEnrollRequest>,
) -> Result<Json<HostTokenResponse>, RouteError> {
    let identity = resolve_user_identity(&headers, true).map_err(unauthorized)?;
    let identity = identity.ok_or_else(|| unauthorized("Missing identity"))?;
    if identity.org_id != org_id {
        return Err(forbidden(
            "Cannot rotate host token for another organization",
        ));
    }
    if !is_org_admin(&identity.role) {
        return Err(forbidden("Insufficient role for host token rotation"));
    }

    let enrollment = state
        .auth_store()
        .rotate_host_token(&org_id, &req.host_id)
        .await
        .map_err(bad_request)?;

    let expires_in_hours = req.expires_in_hours.unwrap_or(24 * 30).clamp(1, 24 * 365);
    let (token, exp) = issue_host_jwt(
        &enrollment.org_id,
        &enrollment.host_id,
        enrollment.token_version,
        expires_in_hours,
    )
    .map_err(internal_error)?;

    Ok(Json(HostTokenResponse {
        host_id: enrollment.host_id,
        org_id: enrollment.org_id,
        token,
        token_version: enrollment.token_version,
        enabled: enrollment.enabled,
        expires_at: format_expiry(exp),
    }))
}

async fn disable_host(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(org_id): Path<String>,
    Json(req): Json<HostDisableRequest>,
) -> Result<Json<HostTokenResponse>, RouteError> {
    let identity = resolve_user_identity(&headers, true).map_err(unauthorized)?;
    let identity = identity.ok_or_else(|| unauthorized("Missing identity"))?;
    if identity.org_id != org_id {
        return Err(forbidden("Cannot disable host in another organization"));
    }
    if !is_org_admin(&identity.role) {
        return Err(forbidden("Insufficient role for host disable"));
    }

    let enrollment = state
        .auth_store()
        .disable_host(&org_id, &req.host_id)
        .await
        .map_err(bad_request)?;

    Ok(Json(HostTokenResponse {
        host_id: enrollment.host_id,
        org_id: enrollment.org_id,
        token: String::new(),
        token_version: enrollment.token_version,
        enabled: enrollment.enabled,
        expires_at: Utc::now().to_rfc3339(),
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/me", get(me))
        .route("/api/v1/orgs", get(list_orgs).post(create_org))
        .route("/api/v1/orgs/{orgId}/hosts/enroll", post(enroll_host))
        .route(
            "/api/v1/orgs/{orgId}/hosts/rotate-token",
            post(rotate_host_token),
        )
        .route("/api/v1/orgs/{orgId}/hosts/disable", post(disable_host))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use serde_json::{json, Value};
    use tempfile::TempDir;
    use tower::ServiceExt;
    use vk_core::{kanban::KanbanStore, task::FileTaskStore};

    use crate::{gateway::GatewayManager, state::AppState};

    async fn build_state() -> (AppState, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let tasks_path = data_dir.join("tasks.json");
        let task_store = Arc::new(FileTaskStore::new(tasks_path).await.unwrap());
        let kanban_path = data_dir.join("kanban.json");
        let kanban_store = Arc::new(
            KanbanStore::with_task_store(kanban_path, Arc::clone(&task_store))
                .await
                .unwrap(),
        );
        let gateway_manager = Arc::new(GatewayManager::with_stores(
            Arc::clone(&task_store),
            Arc::clone(&kanban_store),
        ));

        let state = AppState::with_stores(
            data_dir,
            Arc::clone(&task_store),
            Arc::clone(&kanban_store),
            Arc::clone(&gateway_manager),
        )
        .await
        .unwrap();

        (state, temp_dir)
    }

    #[tokio::test]
    async fn register_and_login_return_jwt() {
        let (state, _tmp) = build_state().await;
        let app = super::router().with_state(state);

        let register_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/register")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "email": "dev@example.com",
                            "password": "dev-pass",
                            "name": "Dev User",
                            "orgName": "Dev Org"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(register_response.status(), StatusCode::CREATED);

        let login_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/login")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "email": "dev@example.com",
                            "password": "dev-pass"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(login_response.status(), StatusCode::OK);

        let body = to_bytes(login_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert!(payload["token"].is_string());
        assert!(payload["orgId"].is_string());
    }

    #[tokio::test]
    async fn enroll_host_returns_host_token() {
        let (state, _tmp) = build_state().await;
        let app = super::router().with_state(state);

        let register_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/register")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "email": "ops@example.com",
                            "password": "ops-pass",
                            "name": "Ops User",
                            "orgName": "Ops Org"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(register_response.status(), StatusCode::CREATED);
        let register_body = to_bytes(register_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let register_payload: Value = serde_json::from_slice(&register_body).unwrap();
        let token = register_payload["token"].as_str().unwrap();
        let org_id = register_payload["orgId"].as_str().unwrap();

        let enroll_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/orgs/{}/hosts/enroll", org_id))
                    .header("Authorization", format!("Bearer {}", token))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "hostId": "host-test"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(enroll_response.status(), StatusCode::OK);
        let enroll_body = to_bytes(enroll_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let enroll_payload: Value = serde_json::from_slice(&enroll_body).unwrap();
        assert_eq!(enroll_payload["hostId"], "host-test");
        assert_eq!(enroll_payload["orgId"], org_id);
        assert!(enroll_payload["token"].is_string());
    }
}
