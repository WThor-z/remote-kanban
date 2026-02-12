//! Workspace API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Deserializer};
use std::fs;
use std::collections::HashSet;
use std::path::{Component, Path as FsPath, PathBuf};
use uuid::Uuid;

use vk_core::project::CreateProjectRequest;
use vk_core::task::TaskRepository;

use crate::state::AppState;
use vk_core::workspace::{CreateWorkspaceRequest, Workspace, WorkspaceSummary};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub host_id: Option<String>,
    pub root_path: Option<String>,
    #[serde(default, deserialize_with = "deserialize_default_project_id")]
    pub default_project_id: Option<Option<Uuid>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteWorkspaceRequest {
    pub confirm_name: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteWorkspaceResponse {
    pub deleted_project_count: usize,
    pub deleted_task_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceProjectRequest {
    pub name: String,
    pub local_path: String,
    pub remote_url: Option<String>,
    pub default_branch: Option<String>,
    pub worktree_dir: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceProjectResponse {
    pub id: String,
    pub name: String,
    pub local_path: String,
    pub remote_url: Option<String>,
    pub default_branch: String,
    pub gateway_id: String,
    pub workspace_id: String,
    pub worktree_dir: String,
    pub source: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredWorkspaceProject {
    pub name: String,
    pub local_path: String,
    pub source: String,
    pub registered_project_id: Option<String>,
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
    if let Some(host_id) = req.host_id {
        workspace.host_id = host_id;
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

async fn delete_workspace(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<DeleteWorkspaceRequest>,
) -> Result<Json<DeleteWorkspaceResponse>, (StatusCode, String)> {
    let workspace_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid workspace ID".to_string()))?;

    let workspace = state
        .workspace_store()
        .get(workspace_id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Workspace not found".to_string()))?;

    if req.confirm_name != workspace.name {
        return Err((
            StatusCode::CONFLICT,
            "confirmName must exactly match workspace name".to_string(),
        ));
    }

    let projects = state.project_store().list().await;
    let project_ids: HashSet<Uuid> = projects
        .iter()
        .filter(|project| project.workspace_id == workspace_id)
        .map(|project| project.id)
        .collect();

    for project_id in &project_ids {
        state
            .project_store()
            .delete(*project_id)
            .await
            .map_err(map_store_error)?;
    }

    let tasks = state
        .task_store()
        .list()
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    let mut deleted_task_count = 0usize;
    for task in tasks {
        let belongs_to_workspace = task.workspace_id == Some(workspace_id);
        let belongs_to_project = task
            .project_id
            .is_some_and(|project_id| project_ids.contains(&project_id));
        if belongs_to_workspace || belongs_to_project {
            if state
                .task_store()
                .delete(task.id)
                .await
                .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
            {
                deleted_task_count += 1;
            }
        }
    }

    state
        .workspace_store()
        .delete(workspace_id)
        .await
        .map_err(map_store_error)?;

    Ok(Json(DeleteWorkspaceResponse {
        deleted_project_count: project_ids.len(),
        deleted_task_count,
    }))
}

async fn create_workspace_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<CreateWorkspaceProjectRequest>,
) -> Result<(StatusCode, Json<WorkspaceProjectResponse>), (StatusCode, String)> {
    let workspace_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid workspace ID".to_string()))?;

    let workspace = state
        .workspace_store()
        .get(workspace_id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Workspace not found".to_string()))?;

    let gateway_id = resolve_workspace_gateway_id(&state, &workspace).await;
    if gateway_id != workspace.host_id {
        let mut healed_workspace = workspace.clone();
        healed_workspace.host_id = gateway_id.clone();
        state
            .workspace_store()
            .update(healed_workspace)
            .await
            .map_err(map_store_error)?;
    }

    let workspace_root = normalize_path(FsPath::new(&workspace.root_path));
    let requested_path = resolve_project_path(&workspace_root, &req.local_path);
    let normalized_project_path = normalize_path(&requested_path);
    if !is_path_within(&workspace_root, &normalized_project_path) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Project localPath must be under workspace rootPath".to_string(),
        ));
    }

    let normalized_project_path_string = normalized_project_path.to_string_lossy().to_string();
    let mut existing_in_workspace = None;
    for project in state.project_store().list().await {
        if project.gateway_id != gateway_id {
            continue;
        }
        if normalize_path(FsPath::new(&project.local_path)) != normalized_project_path {
            continue;
        }
        if project.workspace_id != workspace.id {
            return Err((
                StatusCode::CONFLICT,
                "Project path is already registered under another workspace".to_string(),
            ));
        }
        existing_in_workspace = Some(project);
        break;
    }

    if let Some(existing) = existing_in_workspace {
        return Ok((
            StatusCode::OK,
            Json(WorkspaceProjectResponse {
                id: existing.id.to_string(),
                name: existing.name,
                local_path: existing.local_path,
                remote_url: existing.remote_url,
                default_branch: existing.default_branch,
                gateway_id: existing.gateway_id,
                workspace_id: existing.workspace_id.to_string(),
                worktree_dir: existing.worktree_dir,
                source: "manual".to_string(),
            }),
        ));
    }

    if normalized_project_path.exists() && !normalized_project_path.is_dir() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Project localPath points to a file; expected a directory path".to_string(),
        ));
    }

    fs::create_dir_all(&normalized_project_path).map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            format!(
                "Failed to create project directory at {}: {}",
                normalized_project_path.display(),
                err
            ),
        )
    })?;

    let created = state
        .project_store()
        .register(
            gateway_id,
            CreateProjectRequest {
                name: req.name,
                local_path: normalized_project_path_string,
                remote_url: req.remote_url,
                default_branch: req.default_branch,
                worktree_dir: req.worktree_dir,
                workspace_id: workspace.id,
            },
        )
        .await
        .map_err(map_store_error)?;

    Ok((
        StatusCode::CREATED,
        Json(WorkspaceProjectResponse {
            id: created.id.to_string(),
            name: created.name,
            local_path: created.local_path,
            remote_url: created.remote_url,
            default_branch: created.default_branch,
            gateway_id: created.gateway_id.clone(),
            workspace_id: created.workspace_id.to_string(),
            worktree_dir: created.worktree_dir,
            source: "manual".to_string(),
        }),
    ))
}

async fn resolve_workspace_gateway_id(state: &AppState, workspace: &Workspace) -> String {
    if workspace.host_id.trim().is_empty() {
        return workspace.host_id.clone();
    }

    let connected_hosts = state.gateway_manager().list_hosts().await;
    if connected_hosts
        .iter()
        .any(|host| host.host_id == workspace.host_id)
    {
        return workspace.host_id.clone();
    }

    if connected_hosts.len() == 1 {
        return connected_hosts[0].host_id.clone();
    }

    workspace.host_id.clone()
}

async fn discover_workspace_projects(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<DiscoveredWorkspaceProject>>, (StatusCode, String)> {
    let workspace_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid workspace ID".to_string()))?;

    let workspace = state
        .workspace_store()
        .get(workspace_id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Workspace not found".to_string()))?;

    let workspace_root = normalize_path(FsPath::new(&workspace.root_path));
    let existing_projects = state.project_store().list().await;
    let existing_map: std::collections::HashMap<String, Uuid> = existing_projects
        .into_iter()
        .filter(|project| project.workspace_id == workspace_id)
        .map(|project| {
            (
                normalize_path(FsPath::new(&project.local_path))
                    .to_string_lossy()
                    .to_string(),
                project.id,
            )
        })
        .collect();

    let mut discovered = discover_git_projects(&workspace_root)
        .into_iter()
        .map(|path| {
            let normalized = normalize_path(&path).to_string_lossy().to_string();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("project")
                .to_string();
            DiscoveredWorkspaceProject {
                name,
                local_path: normalized.clone(),
                source: "discovered".to_string(),
                registered_project_id: existing_map.get(&normalized).map(|id| id.to_string()),
            }
        })
        .collect::<Vec<_>>();
    discovered.sort_by(|a, b| a.local_path.cmp(&b.local_path));

    Ok(Json(discovered))
}

fn resolve_project_path(root: &FsPath, local_path: &str) -> PathBuf {
    let candidate = PathBuf::from(local_path);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn normalize_path(path: &FsPath) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn is_path_within(root: &FsPath, candidate: &FsPath) -> bool {
    candidate.starts_with(root)
}

fn discover_git_projects(root: &FsPath) -> Vec<PathBuf> {
    let mut discovered = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(current) = stack.pop() {
        let git_dir = current.join(".git");
        if git_dir.is_dir() || git_dir.is_file() {
            discovered.push(current);
            continue;
        }

        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }
            let path = entry.path();
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with('.'))
            {
                continue;
            }
            stack.push(path);
        }
    }
    discovered
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
            "/api/workspaces/{id}/projects",
            axum::routing::post(create_workspace_project),
        )
        .route(
            "/api/workspaces/{id}/projects/discover",
            get(discover_workspace_projects),
        )
        .route(
            "/api/workspaces/{id}",
            get(get_workspace).patch(update_workspace).delete(delete_workspace),
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
    use vk_core::{
        kanban::KanbanStore,
        project::CreateProjectRequest,
        task::{FileTaskStore, Task, TaskRepository},
    };

    use super::router;
    use crate::{
        gateway::{protocol::HostCapabilities, GatewayManager},
        state::AppState,
    };

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
        assert!(workspaces[0]["hostId"].is_string());
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
                            "hostId": "host-platform",
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
        assert_eq!(payload["hostId"], "host-platform");
        assert_eq!(payload["rootPath"], "/repos/platform");
        assert!(payload["id"].is_string());
    }

    #[tokio::test]
    async fn create_workspace_rejects_empty_host_id() {
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
                            "hostId": "   ",
                            "rootPath": "/repos/platform"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
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
                            "hostId": "host-platform",
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
                            "hostId": "host-platform",
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
                            "hostId": "host-platform-2",
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
        assert_eq!(payload["hostId"], "host-platform-2");
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
                            "hostId": "host-platform",
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
    async fn update_workspace_rejects_empty_host_id() {
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
                            "hostId": "host-platform",
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
                    .body(Body::from(json!({ "hostId": "   " }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn delete_workspace_requires_exact_confirm_name() {
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
                            "hostId": "host-platform",
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
                    .method("DELETE")
                    .uri(format!("/api/workspaces/{workspace_id}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(json!({ "confirmName": "platform" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn delete_workspace_cascades_projects_and_tasks_and_returns_impact() {
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
                            "hostId": "host-platform",
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
        let workspace_id = uuid::Uuid::parse_str(created_payload["id"].as_str().unwrap()).unwrap();

        let gateway_id = "host-platform".to_string();
        let project_1 = state
            .project_store()
            .register(
                gateway_id.clone(),
                CreateProjectRequest {
                    name: "project-1".to_string(),
                    local_path: "/repos/platform/project-1".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id,
                },
            )
            .await
            .unwrap();
        let project_2 = state
            .project_store()
            .register(
                gateway_id,
                CreateProjectRequest {
                    name: "project-2".to_string(),
                    local_path: "/repos/platform/project-2".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id,
                },
            )
            .await
            .unwrap();

        let other_workspace_id = state.workspace_store().list().await[0].id;
        let other_project = state
            .project_store()
            .register(
                "host-other".to_string(),
                CreateProjectRequest {
                    name: "other-project".to_string(),
                    local_path: "/repos/other/project".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id: other_workspace_id,
                },
            )
            .await
            .unwrap();

        state
            .task_store()
            .create(Task::new("task-1").with_project_binding(project_1.id, workspace_id))
            .await
            .unwrap();
        state
            .task_store()
            .create(Task::new("task-2").with_project_binding(project_2.id, workspace_id))
            .await
            .unwrap();
        let kept_task = state
            .task_store()
            .create(
                Task::new("task-kept")
                    .with_project_binding(other_project.id, other_workspace_id),
            )
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/workspaces/{workspace_id}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(json!({ "confirmName": "Platform" }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["deletedProjectCount"], 2);
        assert_eq!(payload["deletedTaskCount"], 2);

        assert!(state.workspace_store().get(workspace_id).await.is_none());
        assert!(state.project_store().get(project_1.id).await.is_none());
        assert!(state.project_store().get(project_2.id).await.is_none());
        assert!(state.task_store().get(kept_task.id).await.unwrap().is_some());
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

    #[tokio::test]
    async fn create_workspace_project_rejects_path_outside_workspace_root() {
        let (state, _temp_dir) = build_state().await;
        let app = router().with_state(state.clone());
        let host_id = uuid::Uuid::new_v4().to_string();

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
                            "hostId": host_id,
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
                    .method("POST")
                    .uri(format!("/api/workspaces/{workspace_id}/projects"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "name": "outside-project",
                            "localPath": "/repos/other/outside-project"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_workspace_project_binds_to_workspace_host() {
        let (state, temp_dir) = build_state().await;
        let app = router().with_state(state.clone());
        let host_id = uuid::Uuid::new_v4().to_string();
        let workspace_root = temp_dir.path().join("repos").join("platform");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let workspace_root_path = workspace_root.to_string_lossy().to_string();
        let project_path = workspace_root.join("inside-project");
        let project_path_string = project_path.to_string_lossy().to_string();

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
                            "hostId": host_id,
                            "rootPath": workspace_root_path
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let created_body = to_bytes(created.into_body(), usize::MAX).await.unwrap();
        let created_payload: Value = serde_json::from_slice(&created_body).unwrap();
        let workspace_id = uuid::Uuid::parse_str(created_payload["id"].as_str().unwrap()).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/workspaces/{workspace_id}/projects"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "name": "inside-project",
                            "localPath": project_path_string
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let projects = state.project_store().list().await;
        let created_project = projects
            .into_iter()
            .find(|project| project.workspace_id == workspace_id)
            .unwrap();
        assert_eq!(created_project.gateway_id, host_id);
        assert_eq!(created_project.local_path, project_path.to_string_lossy().to_string());
    }

    #[tokio::test]
    async fn create_workspace_project_creates_directory_under_workspace_root() {
        let (state, temp_dir) = build_state().await;
        let app = router().with_state(state.clone());
        let host_id = uuid::Uuid::new_v4().to_string();
        let workspace_root = temp_dir.path().join("repos").join("platform");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let workspace_root_path = workspace_root.to_string_lossy().to_string();
        let project_path = workspace_root.join("inside-project");
        let project_path_string = project_path.to_string_lossy().to_string();

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
                            "hostId": host_id,
                            "rootPath": workspace_root_path
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
                    .method("POST")
                    .uri(format!("/api/workspaces/{workspace_id}/projects"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "name": "inside-project",
                            "localPath": project_path_string
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        assert!(project_path.is_dir());
    }

    #[tokio::test]
    async fn create_workspace_project_accepts_workspace_with_string_host_id() {
        let (state, temp_dir) = build_state().await;
        let app = router().with_state(state.clone());
        let workspace_root = temp_dir.path().join("repos").join("platform");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let workspace_root_path = workspace_root.to_string_lossy().to_string();
        let project_path = workspace_root.join("inside-project");
        let project_path_string = project_path.to_string_lossy().to_string();

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
                            "hostId": "host-platform",
                            "rootPath": workspace_root_path
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
                    .method("POST")
                    .uri(format!("/api/workspaces/{workspace_id}/projects"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "name": "inside-project",
                            "localPath": project_path_string
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
        assert_eq!(payload["gatewayId"], "host-platform");
    }

    #[tokio::test]
    async fn create_workspace_project_rebinds_workspace_host_when_single_gateway_is_connected() {
        let (state, temp_dir) = build_state().await;
        let app = router().with_state(state.clone());
        let workspace_root = temp_dir.path().join("repos").join("platform");
        std::fs::create_dir_all(&workspace_root).unwrap();
        let workspace_root_path = workspace_root.to_string_lossy().to_string();
        let project_path = workspace_root.join("inside-project");
        let project_path_string = project_path.to_string_lossy().to_string();

        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        state
            .gateway_manager()
            .register_host(
                "host-wthor".to_string(),
                HostCapabilities {
                    name: "WThor".to_string(),
                    agents: vec!["opencode".to_string()],
                    max_concurrent: 2,
                    cwd: workspace_root_path.clone(),
                    labels: std::collections::HashMap::new(),
                },
                tx,
            )
            .await;

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
                            "hostId": "local",
                            "rootPath": workspace_root_path
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
                    .method("POST")
                    .uri(format!("/api/workspaces/{workspace_id}/projects"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "name": "inside-project",
                            "localPath": project_path_string
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
        assert_eq!(payload["gatewayId"], "host-wthor");

        let workspace = state
            .workspace_store()
            .get(uuid::Uuid::parse_str(workspace_id).unwrap())
            .await
            .unwrap();
        assert_eq!(workspace.host_id, "host-wthor");
    }

    #[tokio::test]
    async fn create_workspace_project_rejects_existing_gateway_path_in_another_workspace() {
        let (state, _temp_dir) = build_state().await;
        let app = router().with_state(state.clone());
        let host_id = "host-shared".to_string();

        let workspace_a = state
            .workspace_store()
            .create(vk_core::workspace::CreateWorkspaceRequest {
                name: "Workspace A".to_string(),
                slug: Some("workspace-a".to_string()),
                host_id: host_id.clone(),
                root_path: "/repos/shared".to_string(),
                default_project_id: None,
            })
            .await
            .unwrap();

        let workspace_b = state
            .workspace_store()
            .create(vk_core::workspace::CreateWorkspaceRequest {
                name: "Workspace B".to_string(),
                slug: Some("workspace-b".to_string()),
                host_id: host_id.clone(),
                root_path: "/repos/shared".to_string(),
                default_project_id: None,
            })
            .await
            .unwrap();

        state
            .project_store()
            .register(
                host_id.clone(),
                CreateProjectRequest {
                    name: "existing-project".to_string(),
                    local_path: "/repos/shared/project".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id: workspace_a.id,
                },
            )
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/workspaces/{}/projects", workspace_b.id))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "name": "duplicate-project",
                            "localPath": "/repos/shared/project"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn discover_workspace_projects_lists_git_repositories_under_root() {
        let (state, temp_dir) = build_state().await;
        let app = router().with_state(state.clone());
        let host_id = uuid::Uuid::new_v4().to_string();

        let workspace_root = temp_dir.path().join("ws-root");
        let repo_a = workspace_root.join("repo-a");
        let repo_b = workspace_root.join("nested").join("repo-b");
        let repo_c = workspace_root.join("repo-c");
        tokio::fs::create_dir_all(repo_a.join(".git")).await.unwrap();
        tokio::fs::create_dir_all(repo_b.join(".git")).await.unwrap();
        tokio::fs::create_dir_all(&repo_c).await.unwrap();
        tokio::fs::write(repo_c.join(".git"), "gitdir: /tmp/mock").await.unwrap();

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
                            "hostId": host_id,
                            "rootPath": workspace_root.to_string_lossy().to_string()
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
                    .uri(format!("/api/workspaces/{workspace_id}/projects/discover"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let items = payload.as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert!(items.iter().all(|item| item["source"] == "discovered"));
    }
}
