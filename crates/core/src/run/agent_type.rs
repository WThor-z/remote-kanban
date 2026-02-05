use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Supported agent types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    OpenCode,
    ClaudeCode,
    GeminiCli,
    Codex,
}

impl AgentType {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "opencode" => Ok(Self::OpenCode),
            "claude-code" | "claudecode" => Ok(Self::ClaudeCode),
            "gemini-cli" | "geminicli" | "gemini" => Ok(Self::GeminiCli),
            "codex" => Ok(Self::Codex),
            _ => Err(Error::ExecutionFailed(format!("Invalid agent type: {}", s))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpenCode => "opencode",
            Self::ClaudeCode => "claude-code",
            Self::GeminiCli => "gemini-cli",
            Self::Codex => "codex",
        }
    }
}
