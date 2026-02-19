//! Auth routes for orchestrator v1.

use axum::{
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{AuthError, AuthSession, AuthStore, OrgRole, OrganizationSummary, UserSummary};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
    pub org_name: String,
    pub org_slug: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub org_id: Option<Uuid>,
    pub org_slug: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub access_token: String,
    pub token_type: String,
    pub user: UserSummary,
    pub org: OrganizationSummary,
    pub role: OrgRole,
    pub exp: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    pub user: UserSummary,
    pub org: OrganizationSummary,
    pub role: OrgRole,
    pub claims: crate::auth::AuthClaims,
}

async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, Json<ErrorResponse>)> {
    let session = state
        .auth_store()
        .register_owner(
            &req.email,
            &req.password,
            req.display_name,
            &req.org_name,
            req.org_slug,
        )
        .await
        .map_err(map_auth_error)?;

    Ok((
        StatusCode::CREATED,
        Json(build_auth_response(state.auth_store(), session)?),
    ))
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    let session = state
        .auth_store()
        .login(
            &req.email,
            &req.password,
            req.org_id,
            req.org_slug.as_deref(),
        )
        .await
        .map_err(map_auth_error)?;

    Ok(Json(build_auth_response(state.auth_store(), session)?))
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let session = auth_session_from_headers(&state, &headers).await?;
    Ok(Json(MeResponse {
        user: session.user,
        org: session.org,
        role: session.membership.role,
        claims: session.claims,
    }))
}

fn build_auth_response(
    auth_store: &AuthStore,
    session: AuthSession,
) -> Result<AuthResponse, (StatusCode, Json<ErrorResponse>)> {
    let token = auth_store
        .encode_claims(&session.claims)
        .map_err(map_auth_error)?;

    Ok(AuthResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        user: session.user,
        org: session.org,
        role: session.membership.role,
        exp: session.claims.exp,
    })
}

fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, (StatusCode, Json<ErrorResponse>)> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .ok_or_else(|| unauthorized("Missing Authorization header"))?;
    let value = auth_header
        .to_str()
        .map_err(|_| unauthorized("Invalid Authorization header"))?;
    value
        .strip_prefix("Bearer ")
        .ok_or_else(|| unauthorized("Authorization header must use Bearer token"))
}

fn unauthorized(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: message.to_string(),
        }),
    )
}

pub(crate) async fn auth_session_from_headers(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthSession, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(headers)?;
    state
        .auth_store()
        .authorize_bearer(token)
        .await
        .map_err(map_auth_error)
}

pub(crate) fn map_auth_error(error: AuthError) -> (StatusCode, Json<ErrorResponse>) {
    let status = match error {
        AuthError::InvalidInput(_) => StatusCode::BAD_REQUEST,
        AuthError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
        AuthError::Forbidden(_) => StatusCode::FORBIDDEN,
        AuthError::NotFound(_) => StatusCode::NOT_FOUND,
        AuthError::Conflict(_) => StatusCode::CONFLICT,
        AuthError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/me", get(me))
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
        let task_store = Arc::new(
            FileTaskStore::new(data_dir.join("tasks.json"))
                .await
                .unwrap(),
        );
        let kanban_store = Arc::new(
            KanbanStore::with_task_store(data_dir.join("kanban.json"), Arc::clone(&task_store))
                .await
                .unwrap(),
        );
        let gateway_manager = Arc::new(GatewayManager::with_stores(
            Arc::clone(&task_store),
            Arc::clone(&kanban_store),
        ));
        let state = AppState::with_stores(data_dir, task_store, kanban_store, gateway_manager)
            .await
            .unwrap();
        (state, temp_dir)
    }

    #[tokio::test]
    async fn register_and_read_me() {
        let (state, _temp_dir) = build_state().await;
        let app = super::router().with_state(state);

        let register = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/register")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "email": "owner@example.com",
                            "password": "verysecurepw",
                            "orgName": "Acme"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(register.status(), StatusCode::CREATED);
        let register_body = to_bytes(register.into_body(), usize::MAX).await.unwrap();
        let register_payload: Value = serde_json::from_slice(&register_body).unwrap();
        let token = register_payload["accessToken"].as_str().unwrap();

        let me = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/me")
                    .header("Authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(me.status(), StatusCode::OK);
    }
}
