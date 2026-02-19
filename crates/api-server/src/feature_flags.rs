use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureFlagsSnapshot {
    pub multi_tenant: bool,
    pub orchestrator_v1: bool,
    pub memory_enhanced: bool,
}

fn env_flag(name: &str, default: bool) -> bool {
    match std::env::var(name) {
        Ok(raw) => match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default,
        },
        Err(_) => default,
    }
}

pub fn feature_multi_tenant() -> bool {
    env_flag("FEATURE_MULTI_TENANT", false)
}

pub fn feature_orchestrator_v1() -> bool {
    env_flag("FEATURE_ORCHESTRATOR_V1", true)
}

pub fn feature_memory_enhanced() -> bool {
    env_flag("FEATURE_MEMORY_ENHANCED", true)
}

pub fn snapshot() -> FeatureFlagsSnapshot {
    FeatureFlagsSnapshot {
        multi_tenant: feature_multi_tenant(),
        orchestrator_v1: feature_orchestrator_v1(),
        memory_enhanced: feature_memory_enhanced(),
    }
}
