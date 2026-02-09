use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::types::{
    MemoryItem, MemoryItemCreateInput, MemoryItemUpdateInput, MemoryListQuery, MemorySettings,
    MemorySettingsPatch, MemorySource,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct MemoryItemsFile {
    items: Vec<MemoryItem>,
}

pub struct MemoryStore {
    items_path: PathBuf,
    settings_path: PathBuf,
    state: RwLock<MemoryState>,
}

#[derive(Debug, Clone)]
struct MemoryState {
    settings: MemorySettings,
    items: Vec<MemoryItem>,
}

async fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?;
    fs::write(path, bytes).await
}

async fn read_json_or_default<T>(path: &Path, default: T) -> std::io::Result<T>
where
    T: for<'de> Deserialize<'de> + Serialize + Clone,
{
    match fs::read(path).await {
        Ok(bytes) => match serde_json::from_slice::<T>(&bytes) {
            Ok(value) => Ok(value),
            Err(_) => Ok(default),
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            write_json_pretty(path, &default).await?;
            Ok(default)
        }
        Err(err) => Err(err),
    }
}

fn normalize_settings(mut settings: MemorySettings) -> MemorySettings {
    settings.token_budget = settings.token_budget.clamp(200, 6000);
    settings.retrieval_top_k = settings.retrieval_top_k.clamp(1, 50);
    settings
}

fn apply_settings_patch(current: &MemorySettings, patch: &MemorySettingsPatch) -> MemorySettings {
    let mut next = current.clone();
    if let Some(value) = patch.enabled {
        next.enabled = value;
    }
    if let Some(value) = patch.gateway_store_enabled {
        next.gateway_store_enabled = value;
    }
    if let Some(value) = patch.rust_store_enabled {
        next.rust_store_enabled = value;
    }
    if let Some(value) = patch.auto_write {
        next.auto_write = value;
    }
    if let Some(value) = patch.prompt_injection {
        next.prompt_injection = value;
    }
    if let Some(value) = patch.token_budget {
        next.token_budget = value;
    }
    if let Some(value) = patch.retrieval_top_k {
        next.retrieval_top_k = value;
    }
    if let Some(value) = patch.llm_extract_enabled {
        next.llm_extract_enabled = value;
    }
    normalize_settings(next)
}

fn trim_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

impl MemoryStore {
    pub async fn new(root_dir: PathBuf) -> std::io::Result<Self> {
        fs::create_dir_all(&root_dir).await?;
        let items_path = root_dir.join("items.json");
        let settings_path = root_dir.join("settings.json");

        let settings = read_json_or_default(&settings_path, MemorySettings::default()).await?;
        let settings = normalize_settings(settings);
        let items_file =
            read_json_or_default(&items_path, MemoryItemsFile { items: Vec::new() }).await?;

        write_json_pretty(&settings_path, &settings).await?;
        write_json_pretty(
            &items_path,
            &MemoryItemsFile {
                items: items_file.items.clone(),
            },
        )
        .await?;

        Ok(Self {
            items_path,
            settings_path,
            state: RwLock::new(MemoryState {
                settings,
                items: items_file.items,
            }),
        })
    }

    async fn persist_items(&self, items: &[MemoryItem]) -> Result<(), String> {
        write_json_pretty(
            &self.items_path,
            &MemoryItemsFile {
                items: items.to_vec(),
            },
        )
        .await
        .map_err(|err| format!("Failed to persist memory items: {}", err))
    }

    async fn persist_settings(&self, settings: &MemorySettings) -> Result<(), String> {
        write_json_pretty(&self.settings_path, settings)
            .await
            .map_err(|err| format!("Failed to persist memory settings: {}", err))
    }

    pub async fn get_settings(&self) -> MemorySettings {
        self.state.read().await.settings.clone()
    }

    pub async fn update_settings(&self, patch: MemorySettingsPatch) -> Result<MemorySettings, String> {
        let mut state = self.state.write().await;
        let next = apply_settings_patch(&state.settings, &patch);
        self.persist_settings(&next).await?;
        state.settings = next.clone();
        Ok(next)
    }

    pub async fn list_items(&self, query: &MemoryListQuery) -> Vec<MemoryItem> {
        let mut items = self.state.read().await.items.clone();

        if let Some(host_id) = trim_to_none(query.host_id.clone()) {
            items.retain(|item| item.host_id == host_id);
        }
        if let Some(project_id) = trim_to_none(query.project_id.clone()) {
            items.retain(|item| {
                item.scope == super::types::MemoryScope::Host
                    || item.project_id.as_deref() == Some(project_id.as_str())
            });
        }
        if let Some(scope) = query.scope {
            items.retain(|item| item.scope == scope);
        }
        if let Some(kind) = query.kind {
            items.retain(|item| item.kind == kind);
        }
        if query.enabled_only.unwrap_or(false) {
            items.retain(|item| item.enabled);
        }
        if let Some(search) = trim_to_none(query.search.clone()) {
            let needle = search.to_lowercase();
            items.retain(|item| {
                item.content.to_lowercase().contains(&needle)
                    || item
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&needle))
            });
        }

        items.sort_by(|left, right| {
            if left.pinned != right.pinned {
                return right.pinned.cmp(&left.pinned);
            }
            right.updated_at.cmp(&left.updated_at)
        });

        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(50).clamp(1, 500);
        items.into_iter().skip(offset).take(limit).collect()
    }

    pub async fn get_item(&self, id: &str) -> Option<MemoryItem> {
        self.state
            .read()
            .await
            .items
            .iter()
            .find(|item| item.id == id)
            .cloned()
    }

    pub async fn create_item(&self, input: MemoryItemCreateInput) -> Result<MemoryItem, String> {
        let content = input.content.trim().to_string();
        if content.is_empty() {
            return Err("Memory content is required".to_string());
        }

        let now = now_iso();
        let item = MemoryItem {
            id: Uuid::new_v4().to_string(),
            host_id: input.host_id,
            project_id: trim_to_none(input.project_id),
            scope: input.scope,
            kind: input.kind,
            content,
            tags: input.tags,
            confidence: input.confidence.unwrap_or(0.8).clamp(0.0, 1.0),
            pinned: input.pinned.unwrap_or(false),
            enabled: input.enabled.unwrap_or(true),
            source: MemorySource::Manual,
            source_task_id: trim_to_none(input.source_task_id),
            created_at: now.clone(),
            updated_at: now,
            last_used_at: None,
            hit_count: 0,
        };

        let mut state = self.state.write().await;
        state.items.push(item.clone());
        self.persist_items(&state.items).await?;
        Ok(item)
    }

    pub async fn update_item(
        &self,
        id: &str,
        patch: MemoryItemUpdateInput,
    ) -> Result<Option<MemoryItem>, String> {
        let mut state = self.state.write().await;
        let Some(item) = state.items.iter_mut().find(|item| item.id == id) else {
            return Ok(None);
        };

        if let Some(content) = patch.content {
            let trimmed = content.trim().to_string();
            if trimmed.is_empty() {
                return Err("Memory content is required".to_string());
            }
            item.content = trimmed;
        }
        if let Some(scope) = patch.scope {
            item.scope = scope;
        }
        if let Some(kind) = patch.kind {
            item.kind = kind;
        }
        if let Some(tags) = patch.tags {
            item.tags = tags;
        }
        if let Some(confidence) = patch.confidence {
            item.confidence = confidence.clamp(0.0, 1.0);
        }
        if let Some(pinned) = patch.pinned {
            item.pinned = pinned;
        }
        if let Some(enabled) = patch.enabled {
            item.enabled = enabled;
        }
        item.updated_at = now_iso();

        let updated = item.clone();
        self.persist_items(&state.items).await?;
        Ok(Some(updated))
    }

    pub async fn delete_item(&self, id: &str) -> Result<bool, String> {
        let mut state = self.state.write().await;
        let before = state.items.len();
        state.items.retain(|item| item.id != id);
        let deleted = state.items.len() != before;
        if deleted {
            self.persist_items(&state.items).await?;
        }
        Ok(deleted)
    }

    pub async fn upsert_items(&self, items: &[MemoryItem]) -> Result<usize, String> {
        if items.is_empty() {
            return Ok(0);
        }
        let mut state = self.state.write().await;
        let mut changed = 0usize;

        for incoming in items {
            if let Some(existing) = state.items.iter_mut().find(|item| item.id == incoming.id) {
                *existing = incoming.clone();
            } else {
                state.items.push(incoming.clone());
            }
            changed += 1;
        }

        self.persist_items(&state.items).await?;
        Ok(changed)
    }

    pub async fn delete_items(&self, items: &[MemoryItem]) -> Result<usize, String> {
        if items.is_empty() {
            return Ok(0);
        }
        let ids: std::collections::HashSet<String> = items.iter().map(|item| item.id.clone()).collect();
        let mut state = self.state.write().await;
        let before = state.items.len();
        state.items.retain(|item| !ids.contains(&item.id));
        let changed = before.saturating_sub(state.items.len());
        if changed > 0 {
            self.persist_items(&state.items).await?;
        }
        Ok(changed)
    }
}
