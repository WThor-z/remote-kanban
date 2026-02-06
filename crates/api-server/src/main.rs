//! API Server for OpenCode Vibe Kanban
//!
//! This is the main entry point for the Rust backend.
//! It provides REST API on port 8081 and Socket.IO on port 8080.

mod gateway;
mod routes;
mod socket;
mod state;

use axum::Router;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::gateway::{GatewayManager, start_heartbeat_checker};
use crate::socket::{create_socket_layer, SocketState};
use crate::state::AppState;
use vk_core::kanban::KanbanStore;
use vk_core::task::FileTaskStore;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api_server=debug,tower_http=debug,socketioxide=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Determine data directory
    let data_dir = std::env::var("VK_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".vk-data"));

    tracing::info!("Using data directory: {:?}", data_dir);

    // Create TaskStore first (needed by both GatewayManager, KanbanStore and AppState)
    let tasks_path = data_dir.join("tasks.json");
    let task_store = Arc::new(FileTaskStore::new(tasks_path).await
        .expect("Failed to initialize task store"));

    // Create KanbanStore synced with TaskStore
    let kanban_path = data_dir.join("kanban.json");
    let kanban_store = Arc::new(KanbanStore::with_task_store(kanban_path, Arc::clone(&task_store)).await
        .expect("Failed to initialize kanban store"));

    // Create Gateway Manager with TaskStore and KanbanStore for Agent Gateway connections
    let gateway_manager = Arc::new(GatewayManager::with_stores(
        Arc::clone(&task_store),
        Arc::clone(&kanban_store),
    ));
    start_heartbeat_checker(Arc::clone(&gateway_manager));
    tracing::info!("Gateway Manager initialized with TaskStore and KanbanStore");

    // Create application state for REST API (uses shared stores)
    let app_state = AppState::with_stores(
        data_dir.clone(),
        Arc::clone(&task_store),
        Arc::clone(&kanban_store),
        Arc::clone(&gateway_manager),
    )
        .await
        .expect("Failed to initialize application state");

    // Create Socket.IO layer with the shared KanbanStore
    let socket_state = SocketState::new(
        Arc::clone(&kanban_store),
        Arc::clone(&task_store),
        data_dir.clone(),
    );
    let (socket_layer, io) = create_socket_layer(socket_state);

    // Set Socket.IO instance in AppState
    app_state.set_socket_io(io.clone()).await;

// REST API server (port 8081)
    let rest_app = Router::new()
        .merge(routes::health::router())
        .merge(routes::task::router())
        .merge(routes::project::router())
        .merge(routes::executor::router())
        .with_state(app_state.clone())
        .merge(routes::gateway::router(app_state.gateway_manager_arc()))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http());

    // Socket.IO server (port 8080)
    // Layers are applied bottom-to-top, so CorsLayer is added last to be applied first
    let socket_app = Router::new()
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(socket_layer);

    // Start both servers - bind to 0.0.0.0 for localhost/127.0.0.1 compatibility
    let rest_addr = SocketAddr::from(([0, 0, 0, 0], 8081));
    let socket_addr = SocketAddr::from(([0, 0, 0, 0], 8080));

    tracing::info!("REST API listening on {}", rest_addr);
    tracing::info!("Socket.IO listening on {}", socket_addr);

    // Spawn REST server
    let rest_listener = tokio::net::TcpListener::bind(rest_addr).await.unwrap();
    let rest_handle = tokio::spawn(async move {
        axum::serve(rest_listener, rest_app).await.unwrap();
    });

    // Spawn Socket.IO server
    let socket_listener = tokio::net::TcpListener::bind(socket_addr).await.unwrap();
    let socket_handle = tokio::spawn(async move {
        axum::serve(socket_listener, socket_app).await.unwrap();
    });

    // Wait for both
    tokio::try_join!(rest_handle, socket_handle).unwrap();
}
