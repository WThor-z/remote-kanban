//! Workspace persistent store

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::Error;
use crate::Result;

use super::model::{normalize_slug, CreateWorkspaceRequest, Workspace, WorkspaceSummary};

fn default_org_id() -> String {
    std::env::var("VK_DEFAULT_ORG_ID")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "default-org".to_string())
}

#[derive(Clone)]
pub struct WorkspaceStore {
    workspaces: Arc<RwLock<HashMap<Uuid, Workspace>>>,
    file_path: PathBuf,
}

impl WorkspaceStore {
    pub async fn new(file_path: PathBuf) -> Result<Self> {
        let (workspaces, migrated_legacy_workspaces) = if file_path.exists() {
            let content = tokio::fs::read_to_string(&file_path)
                .await
                .map_err(|e| Error::Storage(format!("Failed to read workspaces file: {}", e)))?;

            let loaded: HashMap<Uuid, StoredWorkspace> = serde_json::from_str(&content)
                .map_err(|e| Error::Storage(format!("Failed to parse workspaces file: {}", e)))?;

            let mut migrated = false;
            let workspaces = loaded
                .into_values()
                .map(|workspace| {
                    let host_id = match workspace.host_id {
                        Some(host_id) => host_id,
                        None => {
                            migrated = true;
                            "local".to_string()
                        }
                    };
                    let org_id = match workspace.org_id {
                        Some(org_id) if !org_id.trim().is_empty() => org_id.trim().to_string(),
                        _ => {
                            migrated = true;
                            default_org_id()
                        }
                    };
                    (
                        workspace.id,
                        Workspace {
                            id: workspace.id,
                            name: workspace.name,
                            slug: workspace.slug,
                            org_id,
                            host_id,
                            root_path: workspace.root_path,
                            default_project_id: workspace.default_project_id,
                            created_at: workspace.created_at,
                            updated_at: workspace.updated_at,
                            archived_at: workspace.archived_at,
                        },
                    )
                })
                .collect();
            (workspaces, migrated)
        } else {
            (HashMap::new(), false)
        };

        let store = Self {
            workspaces: Arc::new(RwLock::new(workspaces)),
            file_path,
        };

        if migrated_legacy_workspaces {
            let snapshot = store.workspaces.read().await.clone();
            store.persist_snapshot(&snapshot).await?;
        }

        Ok(store)
    }

    pub async fn create(&self, request: CreateWorkspaceRequest) -> Result<Workspace> {
        let host_id = request.host_id.trim().to_string();
        if host_id.is_empty() {
            return Err(Error::InvalidInput(
                "Workspace hostId cannot be empty".to_string(),
            ));
        }

        let org_id = request
            .org_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(default_org_id);

        let mut workspace = Workspace::new(request.name, host_id, request.root_path);
        workspace.org_id = org_id;
        if let Some(slug) = request.slug {
            let normalized = normalize_slug(&slug)
                .ok_or_else(|| Error::InvalidInput("Workspace slug cannot be empty".to_string()))?;
            workspace = workspace.with_slug(normalized);
        }
        workspace.default_project_id = request.default_project_id;

        let mut workspaces = self.workspaces.write().await;
        if workspaces.values().any(|w| w.slug == workspace.slug) {
            return Err(Error::InvalidInput(format!(
                "Workspace slug '{}' already exists",
                workspace.slug
            )));
        }

        let mut next = workspaces.clone();
        next.insert(workspace.id, workspace.clone());

        self.persist_snapshot(&next).await?;
        *workspaces = next;
        Ok(workspace)
    }

    pub async fn get(&self, id: Uuid) -> Option<Workspace> {
        let workspaces = self.workspaces.read().await;
        workspaces.get(&id).cloned()
    }

    pub async fn list(&self) -> Vec<WorkspaceSummary> {
        let workspaces = self.workspaces.read().await;
        let mut summaries: Vec<_> = workspaces.values().map(WorkspaceSummary::from).collect();
        summaries.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
        summaries
    }

    pub async fn update(&self, workspace: Workspace) -> Result<Workspace> {
        let mut workspaces = self.workspaces.write().await;

        if !workspaces.contains_key(&workspace.id) {
            return Err(Error::NotFound(format!(
                "Workspace {} not found",
                workspace.id
            )));
        }

        let mut updated = workspace;
        updated.slug = normalize_slug(&updated.slug)
            .ok_or_else(|| Error::InvalidInput("Workspace slug cannot be empty".to_string()))?;
        updated.host_id = updated.host_id.trim().to_string();
        if updated.host_id.is_empty() {
            return Err(Error::InvalidInput(
                "Workspace hostId cannot be empty".to_string(),
            ));
        }
        updated.org_id = updated.org_id.trim().to_string();
        if updated.org_id.is_empty() {
            updated.org_id = default_org_id();
        }
        updated.updated_at = Utc::now();
        if workspaces
            .values()
            .any(|existing| existing.id != updated.id && existing.slug == updated.slug)
        {
            return Err(Error::InvalidInput(format!(
                "Workspace slug '{}' already exists",
                updated.slug
            )));
        }

        let mut next = workspaces.clone();
        next.insert(updated.id, updated.clone());

        self.persist_snapshot(&next).await?;
        *workspaces = next;
        Ok(updated)
    }

    pub async fn delete(&self, id: Uuid) -> Result<Option<Workspace>> {
        let mut workspaces = self.workspaces.write().await;
        let Some(removed) = workspaces.get(&id).cloned() else {
            return Ok(None);
        };

        let mut next = workspaces.clone();
        next.remove(&id);

        self.persist_snapshot(&next).await?;
        *workspaces = next;

        Ok(Some(removed))
    }

    async fn persist_snapshot(&self, workspaces: &HashMap<Uuid, Workspace>) -> Result<()> {
        let content = serde_json::to_string_pretty(workspaces)
            .map_err(|e| Error::Storage(format!("Failed to serialize workspaces: {}", e)))?;

        if let Some(parent) = self.file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::Storage(format!("Failed to create directory: {}", e)))?;
        }

        let parent = self.file_path.parent().unwrap_or_else(|| Path::new("."));
        let temp_path = parent.join(format!(".{}.tmp", Uuid::new_v4().as_hyphenated()));

        tokio::fs::write(&temp_path, content)
            .await
            .map_err(|e| Error::Storage(format!("Failed to write temp workspaces file: {}", e)))?;

        let backup_path = parent.join(format!(".{}.bak", Uuid::new_v4().as_hyphenated()));

        let mut had_original = false;
        if tokio::fs::metadata(&self.file_path).await.is_ok() {
            had_original = true;
            if let Err(err) = tokio::fs::rename(&self.file_path, &backup_path).await {
                let _ = tokio::fs::remove_file(&temp_path).await;
                return Err(Error::Storage(format!(
                    "Failed to prepare atomic workspace write: {}",
                    err
                )));
            }
        }

        if let Err(err) = tokio::fs::rename(&temp_path, &self.file_path).await {
            if had_original {
                let _ = tokio::fs::rename(&backup_path, &self.file_path).await;
            }
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(Error::Storage(format!(
                "Failed to finalize atomic workspace write: {}",
                err
            )));
        }

        if had_original {
            let _ = tokio::fs::remove_file(&backup_path).await;
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredWorkspace {
    id: Uuid,
    name: String,
    slug: String,
    org_id: Option<String>,
    host_id: Option<String>,
    root_path: String,
    default_project_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_request(name: &str, slug: Option<&str>) -> CreateWorkspaceRequest {
        CreateWorkspaceRequest {
            name: name.to_string(),
            slug: slug.map(str::to_string),
            org_id: None,
            host_id: "host-1".to_string(),
            root_path: format!("/repos/{}", name.to_lowercase().replace(' ', "-")),
            default_project_id: None,
        }
    }

    #[tokio::test]
    async fn test_workspace_store_create_get_list_update_delete_and_persistence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("workspaces.json");

        let store = WorkspaceStore::new(path.clone()).await.unwrap();

        let created = store
            .create(CreateWorkspaceRequest {
                name: "Platform".to_string(),
                slug: None,
                org_id: None,
                host_id: "host-1".to_string(),
                root_path: "/repos/platform".to_string(),
                default_project_id: None,
            })
            .await
            .unwrap();

        let fetched = store.get(created.id).await;
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, created.id);

        let listed = store.list().await;
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);

        let mut to_update = created.clone();
        to_update.name = "Platform Team".to_string();
        let updated = store.update(to_update).await.unwrap();
        assert_eq!(updated.name, "Platform Team");

        let store2 = WorkspaceStore::new(path).await.unwrap();
        let listed_after_reload = store2.list().await;
        assert_eq!(listed_after_reload.len(), 1);
        assert_eq!(listed_after_reload[0].name, "Platform Team");

        let removed = store2.delete(created.id).await.unwrap();
        assert!(removed.is_some());
        assert!(store2.get(created.id).await.is_none());
    }

    #[tokio::test]
    async fn test_create_normalizes_slug_and_rejects_invalid_or_duplicate() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("workspaces.json");
        let store = WorkspaceStore::new(path).await.unwrap();

        let created = store
            .create(create_request("Platform", Some(" Team Platform ")))
            .await
            .unwrap();
        assert_eq!(created.slug, "team-platform");

        let duplicate = store
            .create(create_request("Another", Some("TEAM___PLATFORM")))
            .await;
        assert!(matches!(duplicate, Err(Error::InvalidInput(_))));

        let invalid = store.create(create_request("Invalid", Some("---"))).await;
        assert!(matches!(invalid, Err(Error::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_create_rejects_empty_host_id() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("workspaces.json");
        let store = WorkspaceStore::new(path).await.unwrap();

        let result = store
            .create(CreateWorkspaceRequest {
                name: "Platform".to_string(),
                slug: None,
                org_id: None,
                host_id: "   ".to_string(),
                root_path: "/repos/platform".to_string(),
                default_project_id: None,
            })
            .await;
        assert!(matches!(result, Err(Error::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_update_rejects_empty_host_id() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("workspaces.json");
        let store = WorkspaceStore::new(path).await.unwrap();

        let created = store
            .create(create_request("Platform", None))
            .await
            .unwrap();
        let mut to_update = created.clone();
        to_update.host_id = "".to_string();

        let result = store.update(to_update).await;
        assert!(matches!(result, Err(Error::InvalidInput(_))));
    }

    #[tokio::test]
    async fn test_update_rejects_duplicate_slug_after_normalization() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("workspaces.json");
        let store = WorkspaceStore::new(path).await.unwrap();

        let first = store
            .create(create_request("Platform", Some("team-platform")))
            .await
            .unwrap();
        let second = store
            .create(create_request("Design", Some("design")))
            .await
            .unwrap();

        let mut to_update = second.clone();
        to_update.slug = "TEAM PLATFORM".to_string();
        let result = store.update(to_update).await;
        assert!(matches!(result, Err(Error::InvalidInput(_))));

        let fetched = store.get(second.id).await.unwrap();
        assert_eq!(fetched.slug, "design");
        assert_eq!(first.slug, "team-platform");
    }

    #[tokio::test]
    async fn test_list_is_deterministic() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("workspaces.json");
        let store = WorkspaceStore::new(path).await.unwrap();

        for idx in 0..10 {
            let name = format!("Workspace {}", idx);
            store.create(create_request(&name, None)).await.unwrap();
        }

        let listed = store.list().await;
        let mut sorted = listed.clone();
        sorted.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
        assert_eq!(
            listed.iter().map(|w| w.id).collect::<Vec<_>>(),
            sorted.iter().map(|w| w.id).collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn test_create_rolls_back_when_persist_fails() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state").join("workspaces.json");
        let store = WorkspaceStore::new(path.clone()).await.unwrap();

        tokio::fs::write(dir.path().join("state"), "not a directory")
            .await
            .unwrap();

        let result = store.create(create_request("Platform", None)).await;
        assert!(matches!(result, Err(Error::Storage(_))));
        assert!(store.list().await.is_empty());
    }

    #[tokio::test]
    async fn test_update_rolls_back_when_persist_fails() {
        let dir = tempdir().unwrap();
        let parent = dir.path().join("state");
        tokio::fs::create_dir_all(&parent).await.unwrap();
        let path = parent.join("workspaces.json");
        let store = WorkspaceStore::new(path.clone()).await.unwrap();

        let created = store
            .create(create_request("Platform", None))
            .await
            .unwrap();

        tokio::fs::remove_file(&path).await.unwrap();
        tokio::fs::remove_dir(&parent).await.unwrap();
        tokio::fs::write(&parent, "not a directory").await.unwrap();

        let mut to_update = created.clone();
        to_update.name = "Platform Team".to_string();
        let result = store.update(to_update).await;
        assert!(matches!(result, Err(Error::Storage(_))));

        let fetched = store.get(created.id).await.unwrap();
        assert_eq!(fetched.name, "Platform");
    }

    #[tokio::test]
    async fn test_delete_rolls_back_when_persist_fails() {
        let dir = tempdir().unwrap();
        let parent = dir.path().join("state");
        tokio::fs::create_dir_all(&parent).await.unwrap();
        let path = parent.join("workspaces.json");
        let store = WorkspaceStore::new(path.clone()).await.unwrap();

        let created = store
            .create(create_request("Platform", None))
            .await
            .unwrap();

        tokio::fs::remove_file(&path).await.unwrap();
        tokio::fs::remove_dir(&parent).await.unwrap();
        tokio::fs::write(&parent, "not a directory").await.unwrap();

        let result = store.delete(created.id).await;
        assert!(matches!(result, Err(Error::Storage(_))));
        assert!(store.get(created.id).await.is_some());
    }
}
