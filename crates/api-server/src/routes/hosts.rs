//! Host enrollment APIs for orchestrator E2.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::host::HostStoreError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollHostRequest {
    pub host_id: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostTokenActionRequest {
    pub host_id: String,
}

async fn list_hosts(
    State(state): State<AppState>,
    Path(org_id): Path<String>,
) -> Result<Json<Vec<crate::host::HostSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let hosts = state
        .host_store()
        .list_hosts(&org_id)
        .await
        .map_err(map_host_error)?;
    Ok(Json(hosts))
}

async fn enroll_host(
    State(state): State<AppState>,
    Path(org_id): Path<String>,
    Json(req): Json<EnrollHostRequest>,
) -> Result<(StatusCode, Json<crate::host::IssuedHostToken>), (StatusCode, Json<ErrorResponse>)> {
    let issued = state
        .host_store()
        .enroll_host(&org_id, &req.host_id, req.name)
        .await
        .map_err(map_host_error)?;
    Ok((StatusCode::CREATED, Json(issued)))
}

async fn rotate_host_token(
    State(state): State<AppState>,
    Path((org_id, host_id)): Path<(String, String)>,
) -> Result<Json<crate::host::IssuedHostToken>, (StatusCode, Json<ErrorResponse>)> {
    let issued = state
        .host_store()
        .rotate_token(&org_id, &host_id)
        .await
        .map_err(map_host_error)?;
    Ok(Json(issued))
}

async fn rotate_host_token_by_body(
    State(state): State<AppState>,
    Path(org_id): Path<String>,
    Json(req): Json<HostTokenActionRequest>,
) -> Result<Json<crate::host::IssuedHostToken>, (StatusCode, Json<ErrorResponse>)> {
    let issued = state
        .host_store()
        .rotate_token(&org_id, &req.host_id)
        .await
        .map_err(map_host_error)?;
    Ok(Json(issued))
}

async fn disable_host(
    State(state): State<AppState>,
    Path((org_id, host_id)): Path<(String, String)>,
) -> Result<Json<crate::host::HostSummary>, (StatusCode, Json<ErrorResponse>)> {
    let host = state
        .host_store()
        .disable_host(&org_id, &host_id)
        .await
        .map_err(map_host_error)?;
    Ok(Json(host))
}

async fn disable_host_by_body(
    State(state): State<AppState>,
    Path(org_id): Path<String>,
    Json(req): Json<HostTokenActionRequest>,
) -> Result<Json<crate::host::HostSummary>, (StatusCode, Json<ErrorResponse>)> {
    let host = state
        .host_store()
        .disable_host(&org_id, &req.host_id)
        .await
        .map_err(map_host_error)?;
    Ok(Json(host))
}

fn map_host_error(err: HostStoreError) -> (StatusCode, Json<ErrorResponse>) {
    let status = match err {
        HostStoreError::InvalidInput(_) => StatusCode::BAD_REQUEST,
        HostStoreError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
        HostStoreError::Forbidden(_) => StatusCode::FORBIDDEN,
        HostStoreError::NotFound(_) => StatusCode::NOT_FOUND,
        HostStoreError::Conflict(_) => StatusCode::CONFLICT,
        HostStoreError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(ErrorResponse {
            error: err.to_string(),
        }),
    )
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/orgs/{org_id}/hosts", get(list_hosts))
        .route(
            "/api/v1/orgs/{org_id}/hosts/enroll",
            axum::routing::post(enroll_host),
        )
        .route(
            "/api/v1/orgs/{org_id}/hosts/rotate-token",
            axum::routing::post(rotate_host_token_by_body),
        )
        .route(
            "/api/v1/orgs/{org_id}/hosts/disable",
            axum::routing::post(disable_host_by_body),
        )
        .route(
            "/api/v1/orgs/{org_id}/hosts/{host_id}/rotate-token",
            axum::routing::post(rotate_host_token),
        )
        .route(
            "/api/v1/orgs/{org_id}/hosts/{host_id}/disable",
            axum::routing::post(disable_host),
        )
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

    use crate::{gateway::GatewayManager, host::HostStoreError, state::AppState};

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
    async fn enroll_rotate_disable_host_end_to_end() {
        let (state, _temp_dir) = build_state().await;
        let app = super::router().with_state(state.clone());
        let org_id = "org-e2";

        let enroll = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/orgs/{org_id}/hosts/enroll"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "hostId": "host-alpha",
                            "name": "Alpha Host"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(enroll.status(), StatusCode::CREATED);
        let enroll_payload: Value =
            serde_json::from_slice(&to_bytes(enroll.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let token_v1 = enroll_payload["token"].as_str().unwrap().to_string();
        assert_eq!(enroll_payload["hostId"], "host-alpha");
        assert_eq!(enroll_payload["orgId"], org_id);
        assert_eq!(enroll_payload["tokenVersion"], 1);

        let rotate = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/orgs/{org_id}/hosts/rotate-token"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(json!({ "hostId": "host-alpha" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(rotate.status(), StatusCode::OK);
        let rotate_payload: Value =
            serde_json::from_slice(&to_bytes(rotate.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let token_v2 = rotate_payload["token"].as_str().unwrap().to_string();
        assert_eq!(rotate_payload["tokenVersion"], 2);

        let old_err = state
            .host_store()
            .verify_connection_token(&token_v1, "host-alpha")
            .await
            .unwrap_err();
        assert!(matches!(old_err, HostStoreError::Unauthorized(_)));

        let verified = state
            .host_store()
            .verify_connection_token(&token_v2, "host-alpha")
            .await
            .unwrap();
        assert_eq!(verified.org_id, org_id);

        let disable = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/orgs/{org_id}/hosts/disable"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(json!({ "hostId": "host-alpha" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(disable.status(), StatusCode::OK);

        let disabled_err = state
            .host_store()
            .verify_connection_token(&token_v2, "host-alpha")
            .await
            .unwrap_err();
        assert!(matches!(disabled_err, HostStoreError::Forbidden(_)));
    }
}
