use serde::{Deserialize, Serialize};

const fn default_memory_recency_half_life_hours() -> u32 {
    72
}

const fn default_memory_hit_count_weight() -> f32 {
    0.15
}

const fn default_memory_pinned_boost() -> f32 {
    1.25
}

const fn default_memory_dedupe_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryScope {
    Project,
    Host,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryKind {
    Preference,
    Constraint,
    Fact,
    Workflow,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    AutoRule,
    AutoLlm,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryItem {
    pub id: String,
    pub host_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub scope: MemoryScope,
    pub kind: MemoryKind,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub confidence: f32,
    pub pinned: bool,
    pub enabled: bool,
    pub source: MemorySource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_task_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<String>,
    pub hit_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySettings {
    pub enabled: bool,
    pub gateway_store_enabled: bool,
    pub rust_store_enabled: bool,
    pub auto_write: bool,
    pub prompt_injection: bool,
    pub token_budget: u32,
    pub retrieval_top_k: u32,
    pub llm_extract_enabled: bool,
    #[serde(default = "default_memory_recency_half_life_hours")]
    pub recency_half_life_hours: u32,
    #[serde(default = "default_memory_hit_count_weight")]
    pub hit_count_weight: f32,
    #[serde(default = "default_memory_pinned_boost")]
    pub pinned_boost: f32,
    #[serde(default = "default_memory_dedupe_enabled")]
    pub dedupe_enabled: bool,
}

impl Default for MemorySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            gateway_store_enabled: true,
            rust_store_enabled: true,
            auto_write: true,
            prompt_injection: true,
            token_budget: 1200,
            retrieval_top_k: 8,
            llm_extract_enabled: true,
            recency_half_life_hours: default_memory_recency_half_life_hours(),
            hit_count_weight: default_memory_hit_count_weight(),
            pinned_boost: default_memory_pinned_boost(),
            dedupe_enabled: default_memory_dedupe_enabled(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MemorySettingsPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gateway_store_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rust_store_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_write: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_injection: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_budget: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieval_top_k: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_extract_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recency_half_life_hours: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hit_count_weight: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_boost: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dedupe_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MemoryListQuery {
    #[serde(default)]
    pub host_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub scope: Option<MemoryScope>,
    #[serde(default)]
    pub kind: Option<MemoryKind>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub enabled_only: Option<bool>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryItemCreateInput {
    pub host_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub scope: MemoryScope,
    pub kind: MemoryKind,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MemoryItemUpdateInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<MemoryScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<MemoryKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HostQuery {
    #[serde(default)]
    pub host_id: Option<String>,
}
