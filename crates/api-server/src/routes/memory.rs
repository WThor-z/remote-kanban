use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::warn;

use crate::gateway::protocol::GatewayMemoryAction;
use crate::memory::{
    HostQuery, MemoryItem, MemoryItemCreateInput, MemoryItemUpdateInput, MemoryListQuery,
    MemorySettings, MemorySettingsPatch,
};
use crate::state::AppState;

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsPatchRequest {
    #[serde(default)]
    host_id: Option<String>,
    #[serde(default)]
    patch: MemorySettingsPatch,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct DeleteItemQuery {
    #[serde(default)]
    host_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteItemResponse {
    deleted: bool,
}

fn error_response(status: StatusCode, message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (status, Json(ErrorResponse { error: message.into() }))
}

fn normalize_host_id(raw: Option<String>) -> Option<String> {
    raw.and_then(|host_id| {
        let trimmed = host_id.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn ensure_any_store_enabled(settings: &MemorySettings) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if !settings.rust_store_enabled && !settings.gateway_store_enabled {
        return Err(error_response(
            StatusCode::CONFLICT,
            "Memory stores are disabled (rustStoreEnabled=false and gatewayStoreEnabled=false)",
        ));
    }
    Ok(())
}

async fn proxy_memory(
    state: &AppState,
    host_id: &str,
    action: GatewayMemoryAction,
    payload: Value,
) -> Result<Value, (StatusCode, Json<ErrorResponse>)> {
    state
        .gateway_manager()
        .request_memory(host_id, action, payload)
        .await
        .map_err(|err| error_response(StatusCode::SERVICE_UNAVAILABLE, err))
}

async fn get_memory_settings(
    State(state): State<AppState>,
    Query(host): Query<HostQuery>,
) -> Result<Json<MemorySettings>, (StatusCode, Json<ErrorResponse>)> {
    let settings = state.memory_store().get_settings().await;

    if !settings.rust_store_enabled && settings.gateway_store_enabled {
        let host_id = normalize_host_id(host.host_id)
            .ok_or_else(|| error_response(StatusCode::BAD_REQUEST, "hostId is required"))?;
        let payload = json!({ "hostId": host_id });
        let data = proxy_memory(&state, &host_id, GatewayMemoryAction::SettingsGet, payload).await?;
        let proxied: MemorySettings = serde_json::from_value(data)
            .map_err(|err| error_response(StatusCode::BAD_GATEWAY, format!("Invalid gateway settings payload: {}", err)))?;
        return Ok(Json(proxied));
    }

    Ok(Json(settings))
}

async fn patch_memory_settings(
    State(state): State<AppState>,
    Json(req): Json<SettingsPatchRequest>,
) -> Result<Json<MemorySettings>, (StatusCode, Json<ErrorResponse>)> {
    let updated = state
        .memory_store()
        .update_settings(req.patch.clone())
        .await
        .map_err(|err| error_response(StatusCode::INTERNAL_SERVER_ERROR, err))?;

    let host_id = normalize_host_id(req.host_id);

    if updated.gateway_store_enabled {
        if !updated.rust_store_enabled && host_id.is_none() {
            return Err(error_response(
                StatusCode::BAD_REQUEST,
                "hostId is required when rustStoreEnabled=false and gatewayStoreEnabled=true",
            ));
        }

        if let Some(host_id) = host_id {
            let payload = json!({
                "hostId": host_id,
                "patch": req.patch,
            });
            let proxied = proxy_memory(&state, &host_id, GatewayMemoryAction::SettingsUpdate, payload).await;
            if !updated.rust_store_enabled {
                let data = proxied?;
                let settings: MemorySettings = serde_json::from_value(data).map_err(|err| {
                    error_response(
                        StatusCode::BAD_GATEWAY,
                        format!("Invalid gateway settings payload: {}", err),
                    )
                })?;
                return Ok(Json(settings));
            }
            if let Err(err) = proxied {
                warn!("Gateway settings mirror failed for host {}: {}", host_id, err.1.error);
            }
        }
    }

    Ok(Json(updated))
}

async fn list_memory_items(
    State(state): State<AppState>,
    Query(query): Query<MemoryListQuery>,
) -> Result<Json<Vec<MemoryItem>>, (StatusCode, Json<ErrorResponse>)> {
    let settings = state.memory_store().get_settings().await;
    ensure_any_store_enabled(&settings)?;

    let host_id = normalize_host_id(query.host_id.clone());

    if !settings.rust_store_enabled && settings.gateway_store_enabled {
        let host_id = host_id.ok_or_else(|| error_response(StatusCode::BAD_REQUEST, "hostId is required"))?;
        let payload = serde_json::to_value(&query)
            .map_err(|err| error_response(StatusCode::BAD_REQUEST, format!("Invalid query payload: {}", err)))?;
        let data = proxy_memory(&state, &host_id, GatewayMemoryAction::ItemsList, payload).await?;
        let items: Vec<MemoryItem> = serde_json::from_value(data).map_err(|err| {
            error_response(
                StatusCode::BAD_GATEWAY,
                format!("Invalid gateway memory items payload: {}", err),
            )
        })?;
        return Ok(Json(items));
    }

    let items = state.memory_store().list_items(&query).await;
    Ok(Json(items))
}

async fn create_memory_item(
    State(state): State<AppState>,
    Json(req): Json<MemoryItemCreateInput>,
) -> Result<Json<MemoryItem>, (StatusCode, Json<ErrorResponse>)> {
    let settings = state.memory_store().get_settings().await;
    ensure_any_store_enabled(&settings)?;

    if !settings.rust_store_enabled && settings.gateway_store_enabled {
        let payload = serde_json::to_value(&req)
            .map_err(|err| error_response(StatusCode::BAD_REQUEST, format!("Invalid request payload: {}", err)))?;
        let data = proxy_memory(&state, &req.host_id, GatewayMemoryAction::ItemsCreate, payload).await?;
        let item: MemoryItem = serde_json::from_value(data).map_err(|err| {
            error_response(
                StatusCode::BAD_GATEWAY,
                format!("Invalid gateway memory item payload: {}", err),
            )
        })?;
        return Ok(Json(item));
    }

    let created = state
        .memory_store()
        .create_item(req.clone())
        .await
        .map_err(|err| error_response(StatusCode::BAD_REQUEST, err))?;

    if settings.gateway_store_enabled {
        let payload = serde_json::to_value(&req)
            .map_err(|err| error_response(StatusCode::BAD_REQUEST, format!("Invalid request payload: {}", err)))?;
        if let Err(err) = proxy_memory(&state, &created.host_id, GatewayMemoryAction::ItemsCreate, payload).await {
            warn!(
                "Gateway create mirror failed for host {}: {}",
                created.host_id, err.1.error
            );
        }
    }

    Ok(Json(created))
}

async fn update_memory_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<MemoryItemUpdateInput>,
) -> Result<Json<MemoryItem>, (StatusCode, Json<ErrorResponse>)> {
    let settings = state.memory_store().get_settings().await;
    ensure_any_store_enabled(&settings)?;

    let local_existing = if settings.rust_store_enabled {
        state.memory_store().get_item(&id).await
    } else {
        None
    };
    let host_id = normalize_host_id(req.host_id.clone())
        .or_else(|| local_existing.as_ref().map(|item| item.host_id.clone()));

    if !settings.rust_store_enabled && settings.gateway_store_enabled {
        let host_id = host_id.ok_or_else(|| error_response(StatusCode::BAD_REQUEST, "hostId is required"))?;
        let payload = json!({
            "id": id,
            "hostId": host_id,
            "content": req.content,
            "scope": req.scope,
            "kind": req.kind,
            "tags": req.tags,
            "confidence": req.confidence,
            "pinned": req.pinned,
            "enabled": req.enabled,
        });
        let data = proxy_memory(&state, &host_id, GatewayMemoryAction::ItemsUpdate, payload).await?;
        let item: Option<MemoryItem> = serde_json::from_value(data).map_err(|err| {
            error_response(
                StatusCode::BAD_GATEWAY,
                format!("Invalid gateway memory item payload: {}", err),
            )
        })?;
        let item = item.ok_or_else(|| error_response(StatusCode::NOT_FOUND, "Memory item not found"))?;
        return Ok(Json(item));
    }

    let updated = state
        .memory_store()
        .update_item(&id, req.clone())
        .await
        .map_err(|err| error_response(StatusCode::BAD_REQUEST, err))?;
    let updated = updated.ok_or_else(|| error_response(StatusCode::NOT_FOUND, "Memory item not found"))?;

    if settings.gateway_store_enabled {
        if let Some(host_id) = host_id {
            let payload = json!({
                "id": id,
                "hostId": host_id,
                "content": req.content,
                "scope": req.scope,
                "kind": req.kind,
                "tags": req.tags,
                "confidence": req.confidence,
                "pinned": req.pinned,
                "enabled": req.enabled,
            });
            if let Err(err) = proxy_memory(&state, &host_id, GatewayMemoryAction::ItemsUpdate, payload).await {
                warn!("Gateway update mirror failed for host {}: {}", host_id, err.1.error);
            }
        } else {
            warn!("Skip gateway memory mirror for item {} because hostId is unknown", id);
        }
    }

    Ok(Json(updated))
}

async fn delete_memory_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DeleteItemQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let settings = state.memory_store().get_settings().await;
    ensure_any_store_enabled(&settings)?;

    let local_existing = if settings.rust_store_enabled {
        state.memory_store().get_item(&id).await
    } else {
        None
    };
    let host_id =
        normalize_host_id(query.host_id).or_else(|| local_existing.as_ref().map(|item| item.host_id.clone()));

    if !settings.rust_store_enabled && settings.gateway_store_enabled {
        let host_id = host_id.ok_or_else(|| error_response(StatusCode::BAD_REQUEST, "hostId is required"))?;
        let payload = json!({
            "id": id,
            "hostId": host_id,
        });
        let data = proxy_memory(&state, &host_id, GatewayMemoryAction::ItemsDelete, payload).await?;
        let result: DeleteItemResponse = serde_json::from_value(data).map_err(|err| {
            error_response(
                StatusCode::BAD_GATEWAY,
                format!("Invalid gateway delete payload: {}", err),
            )
        })?;
        if !result.deleted {
            return Err(error_response(StatusCode::NOT_FOUND, "Memory item not found"));
        }
        return Ok(StatusCode::NO_CONTENT);
    }

    let deleted = state
        .memory_store()
        .delete_item(&id)
        .await
        .map_err(|err| error_response(StatusCode::INTERNAL_SERVER_ERROR, err))?;
    if !deleted {
        return Err(error_response(StatusCode::NOT_FOUND, "Memory item not found"));
    }

    if settings.gateway_store_enabled {
        if let Some(host_id) = host_id {
            let payload = json!({
                "id": id,
                "hostId": host_id,
            });
            if let Err(err) = proxy_memory(&state, &host_id, GatewayMemoryAction::ItemsDelete, payload).await {
                warn!("Gateway delete mirror failed for host {}: {}", host_id, err.1.error);
            }
        } else {
            warn!("Skip gateway memory mirror for delete {} because hostId is unknown", id);
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/memory/settings", get(get_memory_settings))
        .route("/api/memory/settings", patch(patch_memory_settings))
        .route("/api/memory/items", get(list_memory_items))
        .route("/api/memory/items", post(create_memory_item))
        .route("/api/memory/items/{id}", patch(update_memory_item))
        .route("/api/memory/items/{id}", delete(delete_memory_item))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use serde_json::{json, Value};
    use tempfile::TempDir;
    use tower::ServiceExt;
    use vk_core::{
        kanban::KanbanStore,
        task::FileTaskStore,
    };

    use crate::{
        gateway::{
            protocol::{HostCapabilities, ServerToGatewayMessage},
            GatewayManager,
        },
        state::AppState,
    };

    async fn build_state() -> (AppState, Arc<GatewayManager>, TempDir) {
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
        gateway_manager.set_memory_store(state.memory_store_arc()).await;

        (state, gateway_manager, temp_dir)
    }

    #[tokio::test]
    async fn get_settings_returns_defaults() {
        let (state, _manager, _tmp) = build_state().await;
        let app = super::router().with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/memory/settings")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["enabled"], true);
        assert_eq!(payload["tokenBudget"], 1200);
    }

    #[tokio::test]
    async fn memory_crud_works_with_rust_store() {
        let (state, _manager, _tmp) = build_state().await;
        let app = super::router().with_state(state.clone());

        let create_body = json!({
            "hostId": "host-a",
            "projectId": "project-a",
            "scope": "project",
            "kind": "fact",
            "content": "Use pnpm for workspace commands",
            "tags": ["tooling"]
        });

        let created_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/memory/items")
                    .header("Content-Type", "application/json")
                    .body(Body::from(create_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created_resp.status(), StatusCode::OK);
        let created_body = to_bytes(created_resp.into_body(), usize::MAX).await.unwrap();
        let created: Value = serde_json::from_slice(&created_body).unwrap();
        let id = created["id"].as_str().unwrap().to_string();

        let list_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/memory/items?hostId=host-a")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list_resp.status(), StatusCode::OK);
        let list_body = to_bytes(list_resp.into_body(), usize::MAX).await.unwrap();
        let list_payload: Value = serde_json::from_slice(&list_body).unwrap();
        assert!(list_payload.as_array().unwrap().len() >= 1);

        let update_body = json!({
            "hostId": "host-a",
            "content": "Use pnpm for all workspace and service commands",
            "pinned": true
        });
        let update_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/memory/items/{}", id))
                    .header("Content-Type", "application/json")
                    .body(Body::from(update_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(update_resp.status(), StatusCode::OK);

        let delete_resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/memory/items/{}?hostId=host-a", id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn list_returns_conflict_when_all_stores_disabled() {
        let (state, _manager, _tmp) = build_state().await;
        let app = super::router().with_state(state);

        let patch_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/memory/settings")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "patch": {
                                "gatewayStoreEnabled": false,
                                "rustStoreEnabled": false
                            }
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(patch_resp.status(), StatusCode::OK);

        let list_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/memory/items?hostId=host-a")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list_resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn proxy_mode_uses_gateway_when_rust_store_disabled() {
        let (state, manager, _tmp) = build_state().await;
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);

        manager
            .register_host(
                "host-proxy".to_string(),
                HostCapabilities {
                    name: "proxy-host".to_string(),
                    agents: vec!["opencode".to_string()],
                    max_concurrent: 2,
                    cwd: "/tmp".to_string(),
                    labels: HashMap::new(),
                },
                tx,
            )
            .await;

        let manager_clone = Arc::clone(&manager);
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let ServerToGatewayMessage::MemoryRequest { request_id, action, .. } = message {
                    if action == crate::gateway::protocol::GatewayMemoryAction::ItemsList {
                        let payload = json!([{
                            "id": "m-1",
                            "hostId": "host-proxy",
                            "scope": "host",
                            "kind": "preference",
                            "content": "Prefer concise responses",
                            "tags": [],
                            "confidence": 0.9,
                            "pinned": false,
                            "enabled": true,
                            "source": "manual",
                            "createdAt": "2026-02-08T00:00:00Z",
                            "updatedAt": "2026-02-08T00:00:00Z",
                            "hitCount": 0
                        }]);
                        manager_clone
                            .handle_memory_response(&request_id, true, Some(payload), None)
                            .await;
                    } else if action
                        == crate::gateway::protocol::GatewayMemoryAction::SettingsUpdate
                    {
                        let payload = json!({
                            "enabled": true,
                            "gatewayStoreEnabled": true,
                            "rustStoreEnabled": false,
                            "autoWrite": true,
                            "promptInjection": true,
                            "tokenBudget": 1200,
                            "retrievalTopK": 8,
                            "llmExtractEnabled": true
                        });
                        manager_clone
                            .handle_memory_response(&request_id, true, Some(payload), None)
                            .await;
                    }
                }
            }
        });

        let app = super::router().with_state(state);

        let patch_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/memory/settings")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        json!({
                            "hostId": "host-proxy",
                            "patch": {
                                "rustStoreEnabled": false,
                                "gatewayStoreEnabled": true
                            }
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(patch_resp.status(), StatusCode::OK);

        let list_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/memory/items?hostId=host-proxy")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list_resp.status(), StatusCode::OK);
        let list_body = to_bytes(list_resp.into_body(), usize::MAX).await.unwrap();
        let list_payload: Value = serde_json::from_slice(&list_body).unwrap();
        assert_eq!(list_payload.as_array().unwrap().len(), 1);
    }
}
