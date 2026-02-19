//! OpenCode HTTP API Client
//!
//! Communicates with OpenCode CLI in serve mode.

use base64::Engine;
use futures::StreamExt;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, Mutex, RwLock};
use tracing::{debug, error, info, warn};

use crate::error::Error;
use crate::Result;

/// Configuration for OpenCode client
#[derive(Debug, Clone)]
pub struct OpencodeConfig {
    /// Working directory for the agent
    pub cwd: PathBuf,
    /// Additional environment variables
    pub env: Vec<(String, String)>,
}

impl Default for OpencodeConfig {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            env: Vec::new(),
        }
    }
}

/// OpenCode session info
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpencodeSession {
    pub id: String,
    pub base_url: String,
}

/// Event from OpenCode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpencodeEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub properties: serde_json::Value,
}

/// Client state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    Stopped,
    Starting,
    Ready,
    Running,
    Completed,
    Failed,
}

/// OpenCode Client for communicating with OpenCode serve mode
pub struct OpencodeClient {
    config: OpencodeConfig,
    password: String,
    base_url: Arc<RwLock<Option<String>>>,
    process: Arc<Mutex<Option<Child>>>,
    state: Arc<RwLock<ClientState>>,
    event_tx: broadcast::Sender<OpencodeEvent>,
    abort_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl OpencodeClient {
    /// Create a new OpenCode client
    pub fn new(config: OpencodeConfig) -> Self {
        let password = Self::generate_password();
        let (event_tx, _) = broadcast::channel(100);

        Self {
            config,
            password,
            base_url: Arc::new(RwLock::new(None)),
            process: Arc::new(Mutex::new(None)),
            state: Arc::new(RwLock::new(ClientState::Stopped)),
            event_tx,
            abort_tx: Arc::new(Mutex::new(None)),
        }
    }

    fn generate_password() -> String {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..32)
            .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
            .collect()
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<OpencodeEvent> {
        self.event_tx.subscribe()
    }

    /// Get current state
    pub async fn state(&self) -> ClientState {
        *self.state.read().await
    }

    /// Start the OpenCode server
    pub async fn start(&self) -> Result<String> {
        {
            let process = self.process.lock().await;
            if process.is_some() {
                return Err(Error::Agent("OpenCode server already running".into()));
            }
        }

        *self.state.write().await = ClientState::Starting;
        info!("Starting OpenCode server...");

        // On Windows, use opencode.cmd for npm global packages
        #[cfg(windows)]
        let program = "opencode.cmd";
        #[cfg(not(windows))]
        let program = "opencode";

        let mut cmd = Command::new(program);
        cmd.args(["serve", "--hostname", "127.0.0.1", "--port", "0"])
            .current_dir(&self.config.cwd)
            .env("OPENCODE_SERVER_PASSWORD", &self.password)
            .env("NPM_CONFIG_LOGLEVEL", "error")
            .env("NODE_NO_WARNINGS", "1")
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add custom env vars
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        // On Windows, create process in new process group
        #[cfg(windows)]
        {
            #[allow(unused_imports)]
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x00000200); // CREATE_NEW_PROCESS_GROUP
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| Error::Agent(format!("Failed to spawn opencode process: {}", e)))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Agent("Failed to capture stdout".into()))?;

        let base_url = Arc::clone(&self.base_url);
        let state = Arc::clone(&self.state);

        // Spawn task to read stdout and detect server ready
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let mut ready_tx = Some(ready_tx);

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                debug!("OpenCode stdout: {}", line);

                // Detect server startup
                if let Some(url) = Self::extract_server_url(&line) {
                    info!("OpenCode server ready at: {}", url);
                    *base_url.write().await = Some(url.clone());
                    *state.write().await = ClientState::Ready;
                    if let Some(tx) = ready_tx.take() {
                        let _ = tx.send(url);
                    }
                }
            }
        });

        *self.process.lock().await = Some(child);

        // Wait for server to be ready with timeout
        let url = tokio::time::timeout(std::time::Duration::from_secs(60), ready_rx)
            .await
            .map_err(|_| Error::Agent("OpenCode server startup timeout".into()))?
            .map_err(|_| Error::Agent("OpenCode server startup failed".into()))?;

        Ok(url)
    }

    fn extract_server_url(line: &str) -> Option<String> {
        // Match: "opencode server listening on http://127.0.0.1:12345"
        let line_lower = line.to_lowercase();
        if line_lower.contains("listening on") {
            if let Some(idx) = line.find("http://") {
                return Some(line[idx..].trim().to_string());
            }
        }
        None
    }

    /// Stop the server
    pub async fn stop(&self) {
        // Abort any running event stream
        if let Some(tx) = self.abort_tx.lock().await.take() {
            let _ = tx.send(());
        }

        // Kill process
        if let Some(mut child) = self.process.lock().await.take() {
            let _ = child.kill().await;
            info!("OpenCode server stopped");
        }

        *self.base_url.write().await = None;
        *self.state.write().await = ClientState::Stopped;
    }

    /// Wait for server to be healthy
    pub async fn wait_for_health(&self) -> Result<bool> {
        let base_url = self
            .base_url
            .read()
            .await
            .clone()
            .ok_or_else(|| Error::Agent("Server not started".into()))?;

        let client = reqwest::Client::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);

        while std::time::Instant::now() < deadline {
            match client
                .get(format!("{}/global/health", base_url))
                .headers(self.get_headers())
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if data.get("healthy") == Some(&serde_json::Value::Bool(true)) {
                            return Ok(true);
                        }
                    }
                }
                _ => {}
            }
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }

        Ok(false)
    }

    /// Create a session
    pub async fn create_session(&self) -> Result<String> {
        let base_url = self
            .base_url
            .read()
            .await
            .clone()
            .ok_or_else(|| Error::Agent("Server not started".into()))?;

        let client = reqwest::Client::new();
        let directory = self.config.cwd.to_string_lossy();

        let resp = client
            .post(format!(
                "{}/session?directory={}",
                base_url,
                urlencoding::encode(&directory)
            ))
            .headers(self.get_headers())
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| Error::Agent(format!("Failed to create session: {}", e)))?;

        if !resp.status().is_success() {
            return Err(Error::Agent(format!(
                "Failed to create session: HTTP {}",
                resp.status()
            )));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Error::Agent(format!("Failed to parse session response: {}", e)))?;

        data.get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| Error::Agent("Session ID not found in response".into()))
    }

    /// Send a message to a session
    pub async fn send_message(&self, session_id: &str, prompt: &str) -> Result<()> {
        let base_url = self
            .base_url
            .read()
            .await
            .clone()
            .ok_or_else(|| Error::Agent("Server not started".into()))?;

        let client = reqwest::Client::new();
        let directory = self.config.cwd.to_string_lossy();

        let resp = client
            .post(format!(
                "{}/session/{}/message?directory={}",
                base_url,
                session_id,
                urlencoding::encode(&directory)
            ))
            .headers(self.get_headers())
            .json(&serde_json::json!({
                "parts": [{"type": "text", "text": prompt}]
            }))
            .send()
            .await
            .map_err(|e| Error::Agent(format!("Failed to send message: {}", e)))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::Agent(format!("Failed to send message: {}", text)));
        }

        Ok(())
    }

    /// Connect to event stream and process events
    pub async fn connect_event_stream(&self, session_id: &str) -> Result<()> {
        let base_url = self
            .base_url
            .read()
            .await
            .clone()
            .ok_or_else(|| Error::Agent("Server not started".into()))?;

        let directory = self.config.cwd.to_string_lossy();
        let client = reqwest::Client::new();

        // Create abort channel
        let (abort_tx, mut abort_rx) = tokio::sync::oneshot::channel();
        *self.abort_tx.lock().await = Some(abort_tx);

        *self.state.write().await = ClientState::Running;

        let resp = client
            .get(format!(
                "{}/event?directory={}",
                base_url,
                urlencoding::encode(&directory)
            ))
            .headers(self.get_headers())
            .header("Accept", "text/event-stream")
            .send()
            .await
            .map_err(|e| Error::Agent(format!("Failed to connect event stream: {}", e)))?;

        if !resp.status().is_success() {
            return Err(Error::Agent(format!(
                "Failed to connect event stream: HTTP {}",
                resp.status()
            )));
        }

        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        let session_id = session_id.to_string();
        let event_tx = self.event_tx.clone();

        loop {
            tokio::select! {
                _ = &mut abort_rx => {
                    info!("Event stream aborted");
                    break;
                }
                chunk = stream.next() => {
                    match chunk {
                        Some(Ok(bytes)) => {
                            buffer.push_str(&String::from_utf8_lossy(&bytes));

                            // Process complete lines
                            while let Some(newline_pos) = buffer.find('\n') {
                                let line = buffer[..newline_pos].to_string();
                                buffer = buffer[newline_pos + 1..].to_string();

                                if line.starts_with("data: ") {
                                    if let Ok(event) = serde_json::from_str::<OpencodeEvent>(&line[6..]) {
                                        // Filter by session ID
                                        if let Some(event_session_id) = Self::extract_session_id(&event) {
                                            if event_session_id != session_id {
                                                continue;
                                            }
                                        }

                                        debug!("OpenCode event: {:?}", event.event_type);
                                        let _ = event_tx.send(event.clone());

                                        // Check for session idle (completion)
                                        if event.event_type == "session.idle" {
                                            info!("Session completed");
                                            *self.state.write().await = ClientState::Completed;
                                            return Ok(());
                                        }

                                        // Check for session error
                                        if event.event_type == "session.error" {
                                            error!("Session error: {:?}", event.properties);
                                            *self.state.write().await = ClientState::Failed;
                                            return Err(Error::Agent("Session error".into()));
                                        }
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => {
                            warn!("Event stream error: {}", e);
                            break;
                        }
                        None => {
                            info!("Event stream ended");
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn extract_session_id(event: &OpencodeEvent) -> Option<String> {
        let props = &event.properties;

        props
            .get("sessionID")
            .and_then(|v| v.as_str())
            .or_else(|| {
                props
                    .get("info")
                    .and_then(|i| i.get("sessionID"))
                    .and_then(|v| v.as_str())
            })
            .or_else(|| {
                props
                    .get("part")
                    .and_then(|p| p.get("sessionID"))
                    .and_then(|v| v.as_str())
            })
            .map(|s| s.to_string())
    }

    /// Abort a session
    pub async fn abort(&self, session_id: &str) -> Result<()> {
        let base_url = match self.base_url.read().await.clone() {
            Some(url) => url,
            None => return Ok(()),
        };

        let directory = self.config.cwd.to_string_lossy();
        let client = reqwest::Client::new();

        let _ = client
            .post(format!(
                "{}/session/{}/abort?directory={}",
                base_url,
                session_id,
                urlencoding::encode(&directory)
            ))
            .headers(self.get_headers())
            .send()
            .await;

        Ok(())
    }

    /// Run a complete session with a prompt
    pub async fn run(&self, prompt: &str) -> Result<()> {
        // Start server
        self.start().await?;

        // Wait for health
        let healthy = self.wait_for_health().await?;
        if !healthy {
            self.stop().await;
            return Err(Error::Agent("OpenCode server failed health check".into()));
        }

        // Create session
        let session_id = self.create_session().await?;
        info!("Created session: {}", session_id);

        // Emit session event
        let base_url = self.base_url.read().await.clone().unwrap_or_default();
        let _ = self.event_tx.send(OpencodeEvent {
            event_type: "session.created".into(),
            properties: serde_json::json!({
                "sessionID": session_id,
                "baseUrl": base_url,
            }),
        });

        // Connect event stream in background
        let event_stream_handle = {
            let client_base_url = Arc::clone(&self.base_url);
            let client_state = Arc::clone(&self.state);
            let client_abort_tx = Arc::clone(&self.abort_tx);
            let client_event_tx = self.event_tx.clone();
            let client_config_cwd = self.config.cwd.clone();
            let password = self.password.clone();
            let session_id_clone = session_id.clone();

            tokio::spawn(async move {
                // Create a mini client for event stream
                let directory = client_config_cwd.to_string_lossy();
                let client = reqwest::Client::new();

                let credentials = base64::engine::general_purpose::STANDARD
                    .encode(format!("opencode:{}", password));

                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert("Content-Type", "application/json".parse().unwrap());
                headers.insert(
                    "Authorization",
                    format!("Basic {}", credentials).parse().unwrap(),
                );
                headers.insert("x-opencode-directory", directory.parse().unwrap());
                headers.insert("Accept", "text/event-stream".parse().unwrap());

                let base_url = client_base_url.read().await.clone().unwrap_or_default();

                let (abort_tx, mut abort_rx) = tokio::sync::oneshot::channel();
                *client_abort_tx.lock().await = Some(abort_tx);

                *client_state.write().await = ClientState::Running;

                let resp = match client
                    .get(format!(
                        "{}/event?directory={}",
                        base_url,
                        urlencoding::encode(&directory)
                    ))
                    .headers(headers)
                    .send()
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        error!("Failed to connect event stream: {}", e);
                        return;
                    }
                };

                let mut stream = resp.bytes_stream();
                let mut buffer = String::new();

                loop {
                    tokio::select! {
                        _ = &mut abort_rx => {
                            info!("Event stream aborted");
                            break;
                        }
                        chunk = stream.next() => {
                            match chunk {
                                Some(Ok(bytes)) => {
                                    buffer.push_str(&String::from_utf8_lossy(&bytes));

                                    while let Some(newline_pos) = buffer.find('\n') {
                                        let line = buffer[..newline_pos].to_string();
                                        buffer = buffer[newline_pos + 1..].to_string();

                                        if line.starts_with("data: ") {
                                            if let Ok(event) = serde_json::from_str::<OpencodeEvent>(&line[6..]) {
                                                if let Some(event_session_id) = OpencodeClient::extract_session_id(&event) {
                                                    if event_session_id != session_id_clone {
                                                        continue;
                                                    }
                                                }

                                                let _ = client_event_tx.send(event.clone());

                                                if event.event_type == "session.idle" {
                                                    *client_state.write().await = ClientState::Completed;
                                                    return;
                                                }

                                                if event.event_type == "session.error" {
                                                    *client_state.write().await = ClientState::Failed;
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                }
                                Some(Err(e)) => {
                                    warn!("Event stream error: {}", e);
                                    break;
                                }
                                None => {
                                    break;
                                }
                            }
                        }
                    }
                }
            })
        };

        // Wait a bit for event stream to connect
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Send message
        self.send_message(&session_id, prompt).await?;

        // Wait for event stream to complete
        let _ = event_stream_handle.await;

        // Emit done event
        let _ = self.event_tx.send(OpencodeEvent {
            event_type: "session.done".into(),
            properties: serde_json::json!({
                "sessionID": session_id,
            }),
        });

        Ok(())
    }

    fn get_headers(&self) -> reqwest::header::HeaderMap {
        let credentials =
            base64::engine::general_purpose::STANDARD.encode(format!("opencode:{}", self.password));

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert(
            "Authorization",
            format!("Basic {}", credentials).parse().unwrap(),
        );
        headers.insert(
            "x-opencode-directory",
            self.config.cwd.to_string_lossy().parse().unwrap(),
        );
        headers
    }
}

impl Drop for OpencodeClient {
    fn drop(&mut self) {
        // Note: Async cleanup would require tokio::task::block_in_place
        // For now, the process cleanup will happen via process handle drop
    }
}
