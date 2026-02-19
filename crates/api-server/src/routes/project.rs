//! Project API routes

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{resolve_user_identity, UserIdentity};
use crate::feature_flags::feature_multi_tenant;
use crate::state::AppState;
use vk_core::project::ProjectSummary;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListProjectsQuery {
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
    #[serde(default)]
    pub org_id: Option<String>,
}

/// List all projects
async fn list_projects(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListProjectsQuery>,
) -> Result<Json<Vec<ProjectSummary>>, (StatusCode, String)> {
    let identity = resolve_route_identity(&headers)?;
    let projects = state.project_store().list().await;
    let mut resolved = Vec::new();

    let org_filter = identity
        .as_ref()
        .map(|current| current.org_id.as_str())
        .or(query.org_id.as_deref());

    for mut project in projects.into_iter().filter(|project| {
        let org_ok = org_filter.is_none_or(|org_id| project.org_id == org_id);
        query
            .workspace_id
            .is_none_or(|workspace_id| project.workspace_id == workspace_id)
            && org_ok
    }) {
        project.gateway_id = reconcile_project_gateway_with_workspace(
            &state,
            project.id,
            project.workspace_id,
            &project.gateway_id,
        )
        .await;
        resolved.push(project);
    }

    Ok(Json(resolved))
}

/// Project detail response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetailResponse {
    pub id: String,
    pub name: String,
    pub org_id: String,
    pub local_path: String,
    pub remote_url: Option<String>,
    pub default_branch: String,
    pub gateway_id: String,
    pub workspace_id: String,
    pub worktree_dir: String,
}

/// Get a single project by ID
async fn get_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ProjectDetailResponse>, (StatusCode, String)> {
    let identity = resolve_route_identity(&headers)?;
    let project_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid project ID".to_string()))?;

    let mut project = state
        .project_store()
        .get(project_id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Project not found".to_string()))?;
    ensure_project_visible(&project, identity.as_ref())?;

    if let Some(workspace_host_id) =
        workspace_host_id_for_project(&state, project.workspace_id).await
    {
        if workspace_host_id != project.gateway_id {
            project.gateway_id = workspace_host_id;
            project = state
                .project_store()
                .update(project)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }

    Ok(Json(ProjectDetailResponse {
        id: project.id.to_string(),
        name: project.name,
        org_id: project.org_id,
        local_path: project.local_path,
        remote_url: project.remote_url,
        default_branch: project.default_branch,
        gateway_id: project.gateway_id.clone(),
        workspace_id: project.workspace_id.to_string(),
        worktree_dir: project.worktree_dir,
    }))
}

/// Update project request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub default_branch: Option<String>,
    pub worktree_dir: Option<String>,
}

/// Update a project
async fn update_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateProjectRequest>,
) -> Result<Json<ProjectDetailResponse>, (StatusCode, String)> {
    let identity = resolve_route_identity(&headers)?;
    let project_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid project ID".to_string()))?;

    let mut project = state
        .project_store()
        .get(project_id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Project not found".to_string()))?;
    ensure_project_visible(&project, identity.as_ref())?;

    // Apply updates
    if let Some(name) = req.name {
        project.name = name;
    }
    if let Some(branch) = req.default_branch {
        project.default_branch = branch;
    }
    if let Some(dir) = req.worktree_dir {
        project.worktree_dir = dir;
    }

    let updated = state
        .project_store()
        .update(project)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ProjectDetailResponse {
        id: updated.id.to_string(),
        name: updated.name,
        org_id: updated.org_id,
        local_path: updated.local_path,
        remote_url: updated.remote_url,
        default_branch: updated.default_branch,
        gateway_id: updated.gateway_id.clone(),
        workspace_id: updated.workspace_id.to_string(),
        worktree_dir: updated.worktree_dir,
    }))
}

fn resolve_route_identity(
    headers: &HeaderMap,
) -> Result<Option<UserIdentity>, (StatusCode, String)> {
    resolve_user_identity(headers, feature_multi_tenant())
        .map_err(|err| (StatusCode::UNAUTHORIZED, err))
}

fn ensure_project_visible(
    project: &vk_core::project::Project,
    identity: Option<&UserIdentity>,
) -> Result<(), (StatusCode, String)> {
    if let Some(identity) = identity {
        if project.org_id != identity.org_id {
            return Err((StatusCode::NOT_FOUND, "Project not found".to_string()));
        }
    }
    Ok(())
}

async fn workspace_host_id_for_project(state: &AppState, workspace_id: Uuid) -> Option<String> {
    state
        .workspace_store()
        .get(workspace_id)
        .await
        .and_then(|workspace| {
            let trimmed = workspace.host_id.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
}

async fn reconcile_project_gateway_with_workspace(
    state: &AppState,
    project_id: Uuid,
    workspace_id: Uuid,
    current_gateway_id: &str,
) -> String {
    let Some(workspace_host_id) = workspace_host_id_for_project(state, workspace_id).await else {
        return current_gateway_id.to_string();
    };

    if workspace_host_id == current_gateway_id {
        return current_gateway_id.to_string();
    }

    if let Some(mut project) = state.project_store().get(project_id).await {
        project.gateway_id = workspace_host_id.clone();
        if let Ok(updated) = state.project_store().update(project).await {
            return updated.gateway_id;
        }
    }

    workspace_host_id
}

/// Create the project router
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/projects", get(list_projects))
        .route("/api/projects/{id}", get(get_project).put(update_project))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use serde_json::Value;
    use tempfile::TempDir;
    use tower::ServiceExt;
    use uuid::Uuid;
    use vk_core::{
        kanban::KanbanStore, project::CreateProjectRequest, task::FileTaskStore,
        workspace::CreateWorkspaceRequest,
    };

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
    async fn list_projects_includes_workspace_id() {
        let (state, _temp_dir) = build_state().await;
        let workspace = state
            .workspace_store()
            .create(CreateWorkspaceRequest {
                name: "Workspace One".to_string(),
                slug: None,
                host_id: "host-one".to_string(),
                root_path: "/tmp/workspace-one".to_string(),
                default_project_id: None,
                org_id: None,
            })
            .await
            .unwrap();

        let project = state
            .project_store()
            .register(
                "host-one".to_string(),
                CreateProjectRequest {
                    name: "list-project".to_string(),
                    local_path: "/tmp/list-project".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id: workspace.id,
                    org_id: None,
                },
            )
            .await
            .unwrap();

        let app = router().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/projects")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let listed = payload
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == project.id.to_string())
            .unwrap();
        assert_eq!(listed["workspaceId"], workspace.id.to_string());
    }

    #[tokio::test]
    async fn list_projects_supports_workspace_filter() {
        let (state, _temp_dir) = build_state().await;

        let workspace_one = state
            .workspace_store()
            .create(CreateWorkspaceRequest {
                name: "Workspace One".to_string(),
                slug: Some("workspace-one".to_string()),
                host_id: "host-one".to_string(),
                root_path: "/tmp/workspace-one".to_string(),
                default_project_id: None,
                org_id: None,
            })
            .await
            .unwrap();
        let workspace_two = state
            .workspace_store()
            .create(CreateWorkspaceRequest {
                name: "Workspace Two".to_string(),
                slug: Some("workspace-two".to_string()),
                host_id: "host-two".to_string(),
                root_path: "/tmp/workspace-two".to_string(),
                default_project_id: None,
                org_id: None,
            })
            .await
            .unwrap();

        let project_one = state
            .project_store()
            .register(
                "host-one".to_string(),
                CreateProjectRequest {
                    name: "project-one".to_string(),
                    local_path: "/tmp/project-one".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id: workspace_one.id,
                    org_id: None,
                },
            )
            .await
            .unwrap();

        state
            .project_store()
            .register(
                "host-two".to_string(),
                CreateProjectRequest {
                    name: "project-two".to_string(),
                    local_path: "/tmp/project-two".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id: workspace_two.id,
                    org_id: None,
                },
            )
            .await
            .unwrap();

        let app = router().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/projects?workspaceId={}", workspace_one.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let listed = payload.as_array().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0]["id"], project_one.id.to_string());
        assert_eq!(listed[0]["workspaceId"], workspace_one.id.to_string());
    }

    #[tokio::test]
    async fn list_projects_reconciles_gateway_id_with_workspace_host() {
        let (state, _temp_dir) = build_state().await;
        let workspace = state
            .workspace_store()
            .create(CreateWorkspaceRequest {
                name: "Workspace One".to_string(),
                slug: Some("workspace-one".to_string()),
                host_id: "host-wthor".to_string(),
                root_path: "/tmp/workspace-one".to_string(),
                default_project_id: None,
                org_id: None,
            })
            .await
            .unwrap();

        let project = state
            .project_store()
            .register(
                "local".to_string(),
                CreateProjectRequest {
                    name: "project-one".to_string(),
                    local_path: "/tmp/project-one".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id: workspace.id,
                    org_id: None,
                },
            )
            .await
            .unwrap();

        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/projects?workspaceId={}", workspace.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        let listed = payload.as_array().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0]["id"], project.id.to_string());
        assert_eq!(listed[0]["gatewayId"], "host-wthor");

        let persisted = state.project_store().get(project.id).await.unwrap();
        assert_eq!(persisted.gateway_id, "host-wthor");
    }

    #[tokio::test]
    async fn get_project_includes_workspace_id() {
        let (state, _temp_dir) = build_state().await;
        let workspace = state
            .workspace_store()
            .create(CreateWorkspaceRequest {
                name: "Workspace One".to_string(),
                slug: None,
                host_id: "host-one".to_string(),
                root_path: "/tmp/workspace-one".to_string(),
                default_project_id: None,
                org_id: None,
            })
            .await
            .unwrap();

        let project = state
            .project_store()
            .register(
                "host-one".to_string(),
                CreateProjectRequest {
                    name: "detail-project".to_string(),
                    local_path: "/tmp/detail-project".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id: workspace.id,
                    org_id: None,
                },
            )
            .await
            .unwrap();

        let app = router().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/projects/{}", project.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["workspaceId"], workspace.id.to_string());
    }

    #[tokio::test]
    async fn get_project_reconciles_gateway_id_with_workspace_host() {
        let (state, _temp_dir) = build_state().await;
        let workspace = state
            .workspace_store()
            .create(CreateWorkspaceRequest {
                name: "Workspace One".to_string(),
                slug: None,
                host_id: "host-wthor".to_string(),
                root_path: "/tmp/workspace-one".to_string(),
                default_project_id: None,
                org_id: None,
            })
            .await
            .unwrap();

        let project = state
            .project_store()
            .register(
                "local".to_string(),
                CreateProjectRequest {
                    name: "detail-project".to_string(),
                    local_path: "/tmp/detail-project".to_string(),
                    remote_url: None,
                    default_branch: None,
                    worktree_dir: None,
                    workspace_id: workspace.id,
                    org_id: None,
                },
            )
            .await
            .unwrap();

        let app = router().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/projects/{}", project.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["gatewayId"], "host-wthor");

        let persisted = state.project_store().get(project.id).await.unwrap();
        assert_eq!(persisted.gateway_id, "host-wthor");
    }

    #[tokio::test]
    async fn get_project_with_invalid_id_returns_bad_request() {
        let (state, _temp_dir) = build_state().await;

        let app = router().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/projects/not-a-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_project_with_missing_id_returns_not_found() {
        let (state, _temp_dir) = build_state().await;

        let app = router().with_state(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/projects/{}", Uuid::new_v4()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
