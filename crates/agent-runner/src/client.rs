use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use futures::StreamExt;
use reqwest::Client;
use serde::Serialize;
use tokio::sync::mpsc;
use tracing::info;

use crate::error::{ExecutorError, Result};
use crate::event::{AgentEvent, OutputStream};
use crate::parser::create_parser;
use crate::process::AgentType;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExecuteRequest {
    task_id: String,
    prompt: String,
    cwd: String,
    agent_type: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StopRequest {
    task_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InputRequest {
    task_id: String,
    content: String,
}

pub struct WorkerClient {
    client: Client,
    url: String,
}

pub trait WorkerClientApi: Send + Sync {
    fn execute(
        &self,
        task_id: String,
        prompt: String,
        cwd: PathBuf,
        agent_type: AgentType,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;

    fn stop(&self, task_id: String) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;

    fn send_input(
        &self,
        task_id: String,
        content: String,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
}

impl WorkerClient {
    pub fn new(url: String) -> Self {
        Self {
            // Disable proxy for internal worker communication
            client: Client::builder()
                .no_proxy()
                .build()
                .unwrap_or_else(|_| Client::new()),
            url,
        }
    }

    pub async fn execute(
        &self,
        task_id: String,
        prompt: String,
        cwd: PathBuf,
        agent_type: AgentType,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<()> {
        let req = ExecuteRequest {
            task_id: task_id.clone(),
            prompt,
            cwd: cwd.to_string_lossy().to_string(),
            agent_type: format!("{:?}", agent_type).to_lowercase(), // "opencode", etc.
        };

        info!("Sending execution request to worker: {}/execute", self.url);

        let res = self.client
            .post(format!("{}/execute", self.url))
            .json(&req)
            .send()
            .await
            .map_err(|e| ExecutorError::execution_failed(format!("Failed to connect to worker: {}", e)))?;

        if !res.status().is_success() {
            let error_text = res.text().await.unwrap_or_else(|_| String::new());
            return Err(ExecutorError::execution_failed(format!("Worker returned error: {}", error_text)));
        }

        // Handle stream
        let mut stream = res.bytes_stream();
        let mut buffer = String::new();
        let mut final_result = Ok(());
        
        let mut parser = create_parser(agent_type);

        while let Some(item) = stream.next().await {
            let chunk: bytes::Bytes = item.map_err(|e| ExecutorError::execution_failed(format!("Stream error: {}", e)))?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            while let Some(idx) = buffer.find("\n\n") {
                let msg = buffer.drain(..idx+2).collect::<String>();
                let msg = msg.trim();
                
                if msg.starts_with("data: ") {
                    let data = msg.trim_start_matches("data: ");
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        match json["type"].as_str() {
                            Some("log") => {
                                if let Some(content) = json["content"].as_str() {
                                    let event = parser.parse(content, OutputStream::Stdout);
                                    if event_tx.send(event).await.is_err() {
                                        break; // Receiver closed
                                    }
                                }
                            }
                            Some("status") => {
                                if let Some(status) = json["status"].as_str() {
                                    if status == "failed" {
                                        let err_msg = json.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
                                        let _ = event_tx.send(AgentEvent::Error { 
                                            message: err_msg.to_string(), 
                                            recoverable: false 
                                        }).await;
                                        final_result = Err(ExecutorError::execution_failed(format!("Task failed: {}", err_msg)));
                                    }
                                    if status == "completed" {
                                        let _ = event_tx.send(AgentEvent::Completed { 
                                            success: true, 
                                            summary: None 
                                        }).await;
                                        final_result = Ok(());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        final_result
    }

    pub async fn stop(&self, task_id: String) -> Result<()> {
        let req = StopRequest { task_id };
        self.client
            .post(format!("{}/stop", self.url))
            .json(&req)
            .send()
            .await
            .map_err(|e| ExecutorError::execution_failed(format!("Failed to stop task: {}", e)))?;
        Ok(())
    }

    pub async fn send_input(&self, task_id: String, content: String) -> Result<()> {
        let req = InputRequest { task_id, content };
        self.client
            .post(format!("{}/input", self.url))
            .json(&req)
            .send()
            .await
            .map_err(|e| ExecutorError::execution_failed(format!("Failed to send input: {}", e)))?;
        Ok(())
    }
}

impl WorkerClientApi for WorkerClient {
    fn execute(
        &self,
        task_id: String,
        prompt: String,
        cwd: PathBuf,
        agent_type: AgentType,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(self.execute(task_id, prompt, cwd, agent_type, event_tx))
    }

    fn stop(&self, task_id: String) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(self.stop(task_id))
    }

    fn send_input(
        &self,
        task_id: String,
        content: String,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(self.send_input(task_id, content))
    }
}
