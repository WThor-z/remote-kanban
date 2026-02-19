//! Organization and membership routes for orchestrator v1.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::OrgRole;
use crate::state::AppState;

use super::auth::{auth_session_from_headers, map_auth_error, ErrorResponse};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOrgSummary {
    pub org: crate::auth::OrganizationSummary,
    pub role: OrgRole,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrgRequest {
    pub name: String,
    pub slug: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertMemberRequest {
    pub email: String,
    pub role: OrgRole,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyRequest {
    pub name: String,
}

async fn list_orgs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserOrgSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let session = auth_session_from_headers(&state, &headers).await?;
    let entries = state
        .auth_store()
        .list_orgs_for_user(session.user.id)
        .await
        .map_err(map_auth_error)?;

    Ok(Json(
        entries
            .into_iter()
            .map(|(org, role)| UserOrgSummary { org, role })
            .collect(),
    ))
}

async fn create_org(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateOrgRequest>,
) -> Result<(StatusCode, Json<crate::auth::OrganizationSummary>), (StatusCode, Json<ErrorResponse>)>
{
    let session = auth_session_from_headers(&state, &headers).await?;
    let org = state
        .auth_store()
        .create_org_for_user(session.user.id, &req.name, req.slug)
        .await
        .map_err(map_auth_error)?;
    Ok((StatusCode::CREATED, Json(org)))
}

async fn get_org(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(org_id): Path<Uuid>,
) -> Result<Json<UserOrgSummary>, (StatusCode, Json<ErrorResponse>)> {
    let session = auth_session_from_headers(&state, &headers).await?;
    if session.org.id != org_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Cross-org access denied".to_string(),
            }),
        ));
    }

    let (org, role) = state
        .auth_store()
        .get_org_for_user(session.user.id, org_id)
        .await
        .map_err(map_auth_error)?;
    Ok(Json(UserOrgSummary { org, role }))
}

async fn list_members(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(org_id): Path<Uuid>,
) -> Result<Json<Vec<crate::auth::MemberRecord>>, (StatusCode, Json<ErrorResponse>)> {
    let session = auth_session_from_headers(&state, &headers).await?;
    if session.org.id != org_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Cross-org access denied".to_string(),
            }),
        ));
    }

    let members = state
        .auth_store()
        .list_members(session.user.id, org_id)
        .await
        .map_err(map_auth_error)?;
    Ok(Json(members))
}

async fn upsert_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(org_id): Path<Uuid>,
    Json(req): Json<UpsertMemberRequest>,
) -> Result<Json<crate::auth::MemberRecord>, (StatusCode, Json<ErrorResponse>)> {
    let session = auth_session_from_headers(&state, &headers).await?;
    if session.org.id != org_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Cross-org access denied".to_string(),
            }),
        ));
    }

    let member = state
        .auth_store()
        .add_member(session.user.id, org_id, &req.email, req.role)
        .await
        .map_err(map_auth_error)?;
    Ok(Json(member))
}

async fn create_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(org_id): Path<Uuid>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<crate::auth::CreatedApiKey>), (StatusCode, Json<ErrorResponse>)> {
    let session = auth_session_from_headers(&state, &headers).await?;
    if session.org.id != org_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Cross-org access denied".to_string(),
            }),
        ));
    }

    let api_key = state
        .auth_store()
        .create_api_key(session.user.id, org_id, &req.name)
        .await
        .map_err(map_auth_error)?;
    Ok((StatusCode::CREATED, Json(api_key)))
}

async fn list_api_keys(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(org_id): Path<Uuid>,
) -> Result<Json<Vec<crate::auth::ApiKeySummary>>, (StatusCode, Json<ErrorResponse>)> {
    let session = auth_session_from_headers(&state, &headers).await?;
    if session.org.id != org_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Cross-org access denied".to_string(),
            }),
        ));
    }

    let api_keys = state
        .auth_store()
        .list_api_keys(session.user.id, org_id)
        .await
        .map_err(map_auth_error)?;
    Ok(Json(api_keys))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/orgs", get(list_orgs).post(create_org))
        .route("/api/v1/orgs/{org_id}", get(get_org))
        .route(
            "/api/v1/orgs/{org_id}/members",
            get(list_members).post(upsert_member),
        )
        .route(
            "/api/v1/orgs/{org_id}/api-keys",
            get(list_api_keys).post(create_api_key),
        )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use serde_json::Value;
    use tempfile::TempDir;
    use tower::ServiceExt;
    use vk_core::{kanban::KanbanStore, task::FileTaskStore};

    use crate::{gateway::GatewayManager, routes::auth, state::AppState};

    async fn build_state() -> (AppState, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();
        let task_store = Arc::new(FileTaskStore::new(data_dir.join("tasks.json")).await.unwrap());
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
    async fn list_orgs_returns_current_user_orgs() {
        let (state, _temp_dir) = build_state().await;
        let session = state
            .auth_store()
            .register_owner(
                "owner@example.com",
                "verysecurepw",
                Some("Owner".to_string()),
                "Org One",
                Some("org-one".to_string()),
            )
            .await
            .unwrap();
        let token = state.auth_store().encode_claims(&session.claims).unwrap();

        let app = super::router().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/orgs")
                    .header("Authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let items = payload.as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["org"]["slug"], "org-one");
    }

    #[tokio::test]
    async fn cross_org_member_access_is_denied() {
        let (state, _temp_dir) = build_state().await;
        let first = state
            .auth_store()
            .register_owner(
                "owner-a@example.com",
                "verysecurepw",
                Some("Owner A".to_string()),
                "Org A",
                Some("org-a".to_string()),
            )
            .await
            .unwrap();
        let second = state
            .auth_store()
            .register_owner(
                "owner-b@example.com",
                "verysecurepw",
                Some("Owner B".to_string()),
                "Org B",
                Some("org-b".to_string()),
            )
            .await
            .unwrap();
        let token = state.auth_store().encode_claims(&first.claims).unwrap();

        let app = super::router()
            .merge(auth::router())
            .with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/orgs/{}/members", second.org.id))
                    .header("Authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
