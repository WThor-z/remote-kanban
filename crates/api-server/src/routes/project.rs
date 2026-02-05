//! Project API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;
use vk_core::project::ProjectSummary;

/// List all projects
async fn list_projects(State(state): State<AppState>) -> Json<Vec<ProjectSummary>> {
    let projects = state.project_store().list().await;
    Json(projects)
}

/// Project detail response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetailResponse {
    pub id: String,
    pub name: String,
    pub local_path: String,
    pub remote_url: Option<String>,
    pub default_branch: String,
    pub gateway_id: String,
    pub worktree_dir: String,
}

/// Get a single project by ID
async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ProjectDetailResponse>, (StatusCode, String)> {
    let project_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid project ID".to_string()))?;

    let project = state
        .project_store()
        .get(project_id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Project not found".to_string()))?;

    Ok(Json(ProjectDetailResponse {
        id: project.id.to_string(),
        name: project.name,
        local_path: project.local_path,
        remote_url: project.remote_url,
        default_branch: project.default_branch,
        gateway_id: project.gateway_id.to_string(),
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
    Path(id): Path<String>,
    Json(req): Json<UpdateProjectRequest>,
) -> Result<Json<ProjectDetailResponse>, (StatusCode, String)> {
    let project_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid project ID".to_string()))?;

    let mut project = state
        .project_store()
        .get(project_id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Project not found".to_string()))?;

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
        local_path: updated.local_path,
        remote_url: updated.remote_url,
        default_branch: updated.default_branch,
        gateway_id: updated.gateway_id.to_string(),
        worktree_dir: updated.worktree_dir,
    }))
}

/// Create the project router
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/projects", get(list_projects))
        .route(
            "/api/projects/{id}",
            get(get_project).put(update_project),
        )
}
