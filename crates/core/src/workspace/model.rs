//! Workspace model definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub root_path: String,
    pub default_project_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

impl Workspace {
    pub fn new(name: impl Into<String>, root_path: impl Into<String>) -> Self {
        let name = name.into();
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            slug: slugify(&name),
            name,
            root_path: root_path.into(),
            default_project_id: None,
            created_at: now,
            updated_at: now,
            archived_at: None,
        }
    }

    pub fn with_slug(mut self, slug: impl Into<String>) -> Self {
        self.slug = slug.into();
        self.updated_at = Utc::now();
        self
    }

    pub fn archive(mut self) -> Self {
        let now = Utc::now();
        self.archived_at = Some(now);
        self.updated_at = now;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub slug: Option<String>,
    pub root_path: String,
    pub default_project_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummary {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub root_path: String,
    pub default_project_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

impl From<&Workspace> for WorkspaceSummary {
    fn from(workspace: &Workspace) -> Self {
        Self {
            id: workspace.id,
            name: workspace.name.clone(),
            slug: workspace.slug.clone(),
            root_path: workspace.root_path.clone(),
            default_project_id: workspace.default_project_id,
            created_at: workspace.created_at,
            updated_at: workspace.updated_at,
            archived_at: workspace.archived_at,
        }
    }
}

fn slugify(input: &str) -> String {
    let mut slug = String::with_capacity(input.len());
    let mut last_was_dash = false;

    for ch in input.chars().flat_map(|c| c.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }

    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "workspace".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn normalize_slug(input: &str) -> Option<String> {
    let mut slug = String::with_capacity(input.len());
    let mut last_was_dash = false;

    for ch in input.chars().flat_map(|c| c.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }

    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_workspace_defaults() {
        let workspace = Workspace::new("Platform", "/repos/platform");

        assert_eq!(workspace.name, "Platform");
        assert_eq!(workspace.slug, "platform");
        assert_eq!(workspace.root_path, "/repos/platform");
        assert!(workspace.default_project_id.is_none());
        assert!(workspace.archived_at.is_none());
        assert!(workspace.created_at <= workspace.updated_at);
    }

    #[test]
    fn test_workspace_with_slug_and_archive() {
        let workspace = Workspace::new("Platform", "/repos/platform").with_slug("team-platform");
        assert_eq!(workspace.slug, "team-platform");

        let archived = workspace.archive();
        assert!(archived.archived_at.is_some());
        assert!(archived.updated_at >= archived.created_at);
    }

    #[test]
    fn test_normalize_slug_for_user_input() {
        assert_eq!(
            normalize_slug(" Team Platform "),
            Some("team-platform".to_string())
        );
        assert_eq!(
            normalize_slug("TEAM___PLATFORM"),
            Some("team-platform".to_string())
        );
        assert_eq!(normalize_slug("---"), None);
        assert_eq!(normalize_slug(""), None);
    }

    #[test]
    fn test_workspace_serializes_in_camel_case() {
        let workspace = Workspace::new("Platform", "/repos/platform");
        let value = serde_json::to_value(&workspace).unwrap();

        assert_eq!(value["rootPath"], json!("/repos/platform"));
        assert!(value.get("root_path").is_none());
        assert!(value.get("createdAt").is_some());
        assert!(value.get("updatedAt").is_some());
    }
}
