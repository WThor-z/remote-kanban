//! Workspace API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Deserializer};
use uuid::Uuid;

use crate::state::AppState;
use vk_core::workspace::{CreateWorkspaceRequest, Workspace, WorkspaceSummary};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub root_path: Option<String>,
    #[serde(default, deserialize_with = "deserialize_default_project_id")]
    pub default_project_id: Option<Option<Uuid>>,
}

fn deserialize_default_project_id<'de, D>(deserializer: D) -> Result<Option<Option<Uuid>>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(Option::<Uuid>::deserialize(deserializer)?))
}

async fn list_workspaces(State(state): State<AppState>) -> Json<Vec<WorkspaceSummary>> {
    let workspaces = state.workspace_store().list().await;
    Json(workspaces)
}

async fn create_workspace(
    State(state): State<AppState>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Result<(StatusCode, Json<Workspace>), (StatusCode, String)> {
    let created = state
        .workspace_store()
        .create(req)
        .await
        .map_err(map_store_error)?;

    Ok((StatusCode::CREATED, Json(created)))
}

async fn get_workspace(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Workspace>, (StatusCode, String)> {
    let workspace_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid workspace ID".to_string()))?;

    let workspace = state
        .workspace_store()
        .get(workspace_id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Workspace not found".to_string()))?;

    Ok(Json(workspace))
}

async fn update_workspace(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateWorkspaceRequest>,
) -> Result<Json<Workspace>, (StatusCode, String)> {
    let workspace_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid workspace ID".to_string()))?;

    let mut workspace = state
        .workspace_store()
        .get(workspace_id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Workspace not found".to_string()))?;

    if let Some(name) = req.name {
        workspace.name = name;
    }
    if let Some(slug) = req.slug {
        workspace.slug = slug;
    }
    if let Some(root_path) = req.root_path {
        workspace.root_path = root_path;
    }
    if let Some(default_project_id) = req.default_project_id {
        workspace.default_project_id = default_project_id;
    }

    let updated = state
        .workspace_store()
        .update(workspace)
        .await
        .map_err(map_store_error)?;

    Ok(Json(updated))
}

fn map_store_error(err: vk_core::Error) -> (StatusCode, String) {
    match err {
        vk_core::Error::NotFound(message) => (StatusCode::NOT_FOUND, message),
        vk_core::Error::InvalidInput(message) => (StatusCode::BAD_REQUEST, message),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/workspaces", get(list_workspaces).post(create_workspace))
        .route(
            "/api/workspaces/{id}",
            get(get_workspace).patch(update_workspace),
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

    use super::router;
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
    async fn list_workspaces_returns_empty_list_initially() {
        let (state, _temp_dir) = build_state().await;
        let app = router().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/workspaces")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let workspaces = payload
            .as_array()
            .expect("list workspaces response should be an array");

        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0]["name"], "Default Workspace");
        assert_eq!(workspaces[0]["slug"], "default");
        assert!(workspaces[0]["id"].is_string());
        assert!(workspaces[0]["createdAt"].is_string());
        assert!(workspaces[0]["updatedAt"].is_string());
    }

    #[tokio::test]
    async fn create_workspace_returns_created_workspace() {
        let (state, _temp_dir) = build_state().await;
        let app = router().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "name": "Platform",
                            "rootPath": "/repos/platform"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["name"], "Platform");
        assert_eq!(payload["rootPath"], "/repos/platform");
        assert!(payload["id"].is_string());
    }

    #[tokio::test]
    async fn get_workspace_returns_workspace_when_found() {
        let (state, _temp_dir) = build_state().await;
        let app = router().with_state(state.clone());

        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "name": "Platform",
                            "rootPath": "/repos/platform"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let created_body = to_bytes(created.into_body(), usize::MAX).await.unwrap();
        let created_payload: Value = serde_json::from_slice(&created_body).unwrap();
        let workspace_id = created_payload["id"].as_str().unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/workspaces/{workspace_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["id"], workspace_id);
        assert_eq!(payload["name"], "Platform");
    }

    #[tokio::test]
    async fn update_workspace_patches_mutable_fields() {
        let (state, _temp_dir) = build_state().await;
        let app = router().with_state(state.clone());

        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "name": "Platform",
                            "rootPath": "/repos/platform"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let created_body = to_bytes(created.into_body(), usize::MAX).await.unwrap();
        let created_payload: Value = serde_json::from_slice(&created_body).unwrap();
        let workspace_id = created_payload["id"].as_str().unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/workspaces/{workspace_id}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "name": "Platform Team",
                            "slug": "platform-team",
                            "rootPath": "/repos/platform-team"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["name"], "Platform Team");
        assert_eq!(payload["slug"], "platform-team");
        assert_eq!(payload["rootPath"], "/repos/platform-team");
    }

    #[tokio::test]
    async fn update_workspace_patch_can_clear_default_project_id() {
        let (state, _temp_dir) = build_state().await;
        let app = router().with_state(state.clone());

        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "name": "Platform",
                            "rootPath": "/repos/platform"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let created_body = to_bytes(created.into_body(), usize::MAX).await.unwrap();
        let created_payload: Value = serde_json::from_slice(&created_body).unwrap();
        let workspace_id = created_payload["id"].as_str().unwrap();

        let default_project_id = uuid::Uuid::new_v4();
        let set_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/workspaces/{workspace_id}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({ "defaultProjectId": default_project_id }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(set_response.status(), StatusCode::OK);
        let set_body = to_bytes(set_response.into_body(), usize::MAX).await.unwrap();
        let set_payload: Value = serde_json::from_slice(&set_body).unwrap();
        assert_eq!(set_payload["defaultProjectId"], default_project_id.to_string());

        let clear_response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/workspaces/{workspace_id}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(json!({ "defaultProjectId": null }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(clear_response.status(), StatusCode::OK);
        let clear_body = to_bytes(clear_response.into_body(), usize::MAX).await.unwrap();
        let clear_payload: Value = serde_json::from_slice(&clear_body).unwrap();
        assert_eq!(clear_payload["defaultProjectId"], Value::Null);
    }

    #[tokio::test]
    async fn invalid_uuid_returns_bad_request() {
        let (state, _temp_dir) = build_state().await;
        let app = router().with_state(state);

        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/workspaces/not-a-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_response.status(), StatusCode::BAD_REQUEST);

        let patch_response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/workspaces/not-a-uuid")
                    .header("Content-Type", "application/json")
                    .body(Body::from(json!({ "name": "Updated" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(patch_response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn missing_workspace_returns_not_found() {
        let (state, _temp_dir) = build_state().await;
        let app = router().with_state(state);
        let missing_id = uuid::Uuid::new_v4();

        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/workspaces/{missing_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_response.status(), StatusCode::NOT_FOUND);

        let patch_response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/workspaces/{missing_id}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(json!({ "name": "Updated" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(patch_response.status(), StatusCode::NOT_FOUND);
    }
}
