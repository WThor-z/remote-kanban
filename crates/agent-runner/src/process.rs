//! Agent process management

use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::error::{ExecutorError, Result};
use crate::event::{AgentEvent, OutputStream};

/// Supported agent types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    OpenCode,
    ClaudeCode,
    GeminiCli,
    Codex,
}

impl AgentType {
    /// Parse agent type from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "opencode" => Ok(Self::OpenCode),
            "claude-code" | "claudecode" => Ok(Self::ClaudeCode),
            "gemini-cli" | "geminicli" | "gemini" => Ok(Self::GeminiCli),
            "codex" => Ok(Self::Codex),
            _ => Err(ExecutorError::InvalidAgentType {
                agent_type: s.to_string(),
            }),
        }
    }

    /// Get the command to run this agent
    pub fn command(&self) -> &'static str {
        match self {
            Self::OpenCode => {
                if cfg!(target_os = "windows") {
                    "opencode.cmd"
                } else {
                    "opencode"
                }
            },
            Self::ClaudeCode => {
                if cfg!(target_os = "windows") {
                    "claude.cmd"
                } else {
                    "claude"
                }
            },
            Self::GeminiCli => "gemini",
            Self::Codex => "codex",
        }
    }

    /// Get default arguments for the agent
    pub fn default_args(&self) -> Vec<&'static str> {
        match self {
            Self::OpenCode => vec!["--non-interactive"],
            Self::ClaudeCode => vec!["--yes"],
            Self::GeminiCli => vec![],
            Self::Codex => vec!["--yes"],
        }
    }
}

/// Configuration for an agent process
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Type of agent to run
    pub agent_type: AgentType,
    /// Working directory for the agent
    pub working_dir: std::path::PathBuf,
    /// The prompt/task to send to the agent
    pub prompt: String,
    /// Additional environment variables
    pub env: Vec<(String, String)>,
    /// Timeout in seconds (0 = no timeout)
    pub timeout_seconds: u64,
}

/// Represents a running agent process
pub struct AgentProcess {
    /// The child process
    child: Child,
    /// Agent type
    agent_type: AgentType,
    /// Event sender
    event_tx: mpsc::Sender<AgentEvent>,
}

impl AgentProcess {
    /// Spawn a new agent process
    pub async fn spawn(
        config: AgentConfig,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<Self> {
        let command = config.agent_type.command();
        let args = config.agent_type.default_args();

        info!(
            "Spawning {} in {:?} with prompt: {}",
            command,
            config.working_dir,
            config.prompt.chars().take(100).collect::<String>()
        );

        // Build the command
        let mut cmd = Command::new(command);
        cmd.current_dir(&config.working_dir)
            .args(&args)
            .arg(&config.prompt)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // Add environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Spawn the process
        let child = cmd.spawn().map_err(|e| {
            ExecutorError::spawn_failed_with_source(
                format!("Failed to spawn {}: {}", command, e),
                e,
            )
        })?;

        Ok(Self {
            child,
            agent_type: config.agent_type,
            event_tx,
        })
    }

    /// Start reading output from the process
    pub async fn start_output_reader(mut self) -> Result<OutputReaderHandle> {
        let stdout = self
            .child
            .stdout
            .take()
            .ok_or_else(|| ExecutorError::spawn_failed("Failed to capture stdout"))?;

        let stderr = self
            .child
            .stderr
            .take()
            .ok_or_else(|| ExecutorError::spawn_failed("Failed to capture stderr"))?;

        let event_tx = self.event_tx.clone();

        // Spawn stdout reader
        let stdout_tx = event_tx.clone();
        let stdout_handle = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                debug!("stdout: {}", line);

                // Parse the line into an event
                let event = parse_agent_output(&line, OutputStream::Stdout);

                if stdout_tx.send(event).await.is_err() {
                    warn!("Event channel closed, stopping stdout reader");
                    break;
                }
            }
        });

        // Spawn stderr reader
        let stderr_tx = event_tx.clone();
        let stderr_handle = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                debug!("stderr: {}", line);

                let event = AgentEvent::RawOutput {
                    stream: OutputStream::Stderr,
                    content: line,
                };

                if stderr_tx.send(event).await.is_err() {
                    warn!("Event channel closed, stopping stderr reader");
                    break;
                }
            }
        });

        Ok(OutputReaderHandle {
            child: self.child,
            stdout_handle,
            stderr_handle,
            agent_type: self.agent_type,
        })
    }
}

/// Handle for the output reader tasks
pub struct OutputReaderHandle {
    child: Child,
    stdout_handle: tokio::task::JoinHandle<()>,
    stderr_handle: tokio::task::JoinHandle<()>,
    #[allow(dead_code)]
    agent_type: AgentType,
}

impl OutputReaderHandle {
    /// Wait for the process to complete
    pub async fn wait(mut self) -> Result<i32> {
        let status = self.child.wait().await?;

        // Wait for readers to finish
        let _ = self.stdout_handle.await;
        let _ = self.stderr_handle.await;

        Ok(status.code().unwrap_or(-1))
    }

    /// Kill the process
    pub async fn kill(mut self) -> Result<()> {
        self.child.kill().await?;
        self.stdout_handle.abort();
        self.stderr_handle.abort();
        Ok(())
    }

    /// Get the process ID
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }
}

/// Parse agent output into an event
fn parse_agent_output(line: &str, stream: OutputStream) -> AgentEvent {
    // Try to parse as JSON first (for structured output)
    if let Ok(event) = serde_json::from_str::<AgentEvent>(line) {
        return event;
    }

    // Check for common patterns
    if line.starts_with("Thinking:") || line.starts_with("💭") {
        return AgentEvent::Thinking {
            content: line.trim_start_matches("Thinking:").trim().to_string(),
        };
    }

    if line.starts_with("Running:") || line.starts_with("$") || line.starts_with(">") {
        return AgentEvent::Command {
            command: line
                .trim_start_matches("Running:")
                .trim_start_matches('$')
                .trim_start_matches('>')
                .trim()
                .to_string(),
            output: String::new(),
            exit_code: None,
        };
    }

    if line.contains("Error:") || line.contains("error:") {
        return AgentEvent::Error {
            message: line.to_string(),
            recoverable: !line.contains("fatal"),
        };
    }

    if line.contains("Complete") || line.contains("Done") || line.contains("Finished") {
        return AgentEvent::Completed {
            success: true,
            summary: Some(line.to_string()),
        };
    }

    // Default to raw output
    AgentEvent::RawOutput {
        stream,
        content: line.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_from_str() {
        assert_eq!(AgentType::from_str("opencode").unwrap(), AgentType::OpenCode);
        assert_eq!(AgentType::from_str("claude-code").unwrap(), AgentType::ClaudeCode);
        assert_eq!(AgentType::from_str("gemini-cli").unwrap(), AgentType::GeminiCli);
        assert_eq!(AgentType::from_str("codex").unwrap(), AgentType::Codex);
        assert!(AgentType::from_str("unknown").is_err());
    }

    #[test]
    fn test_parse_agent_output() {
        let thinking = parse_agent_output("Thinking: about the problem", OutputStream::Stdout);
        assert!(matches!(thinking, AgentEvent::Thinking { .. }));

        let command = parse_agent_output("$ ls -la", OutputStream::Stdout);
        assert!(matches!(command, AgentEvent::Command { .. }));

        let error = parse_agent_output("Error: something went wrong", OutputStream::Stdout);
        assert!(matches!(error, AgentEvent::Error { .. }));

        let raw = parse_agent_output("some random output", OutputStream::Stdout);
        assert!(matches!(raw, AgentEvent::RawOutput { .. }));
    }
}
