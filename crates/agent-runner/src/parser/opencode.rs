use super::OutputParser;
use crate::event::{AgentEvent, OutputStream};

/// Parser for OpenCode CLI output
pub struct OpenCodeParser;

impl OpenCodeParser {
    pub fn new() -> Self {
        Self
    }
}

impl OutputParser for OpenCodeParser {
    fn parse(&mut self, line: &str, stream: OutputStream) -> AgentEvent {
        // 1. Try JSON parsing first (ideal for structured output)
        if line.trim().starts_with('{') && line.trim().ends_with('}') {
            if let Ok(event) = serde_json::from_str::<AgentEvent>(line) {
                return event;
            }
        }

        // 2. Heuristic parsing for human-readable output
        let line_trim = line.trim();

        // Thinking/Reasoning
        if line_trim.starts_with("Thinking:") || line_trim.starts_with("💭") {
            return AgentEvent::Thinking {
                content: line
                    .trim_start_matches("Thinking:")
                    .trim_start_matches("💭")
                    .trim()
                    .to_string(),
            };
        }

        // Command Execution
        // Matches: "Running: ls -la", "$ ls -la", "> ls -la"
        if line_trim.starts_with("Running:")
            || line_trim.starts_with("$ ")
            || line_trim.starts_with("> ")
        {
            let command = if line_trim.starts_with("Running:") {
                line.trim_start_matches("Running:")
            } else if line_trim.starts_with("$ ") {
                line.trim_start_matches("$ ")
            } else {
                line.trim_start_matches("> ")
            };

            return AgentEvent::Command {
                command: command.trim().to_string(),
                output: String::new(), // Output comes in subsequent lines usually
                exit_code: None,
            };
        }

        // File Changes
        // Matches: "Created file: src/main.rs", "Modified: Cargo.toml"
        if line_trim.starts_with("Created file:") {
            let path = line.trim_start_matches("Created file:").trim();
            return AgentEvent::FileChange {
                path: path.to_string(),
                action: crate::event::FileAction::Created,
                diff: None,
            };
        }

        // Error reporting
        if line_trim.starts_with("Error:") || line_trim.to_lowercase().contains("error:") {
            return AgentEvent::Error {
                message: line.to_string(),
                recoverable: !line.to_lowercase().contains("fatal"),
            };
        }

        // Completion
        if line_trim.starts_with("Task Completed") || line_trim.contains("Mission Accomplished") {
            return AgentEvent::Completed {
                success: true,
                summary: Some(line.to_string()),
            };
        }

        // Default: Raw Output
        AgentEvent::RawOutput {
            stream,
            content: line.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{FileAction, OutputStream};

    #[test]
    fn test_parse_thinking() {
        let mut parser = OpenCodeParser::new();
        let event = parser.parse("Thinking: I should check the file", OutputStream::Stdout);
        match event {
            AgentEvent::Thinking { content } => assert_eq!(content, "I should check the file"),
            _ => panic!("Expected Thinking event"),
        }
    }

    #[test]
    fn test_parse_command() {
        let mut parser = OpenCodeParser::new();
        let event = parser.parse("$ cargo build", OutputStream::Stdout);
        match event {
            AgentEvent::Command { command, .. } => assert_eq!(command, "cargo build"),
            _ => panic!("Expected Command event"),
        }
    }

    #[test]
    fn test_parse_json() {
        let mut parser = OpenCodeParser::new();
        let json = r#"{"type":"message","content":"Hello world"}"#;
        let event = parser.parse(json, OutputStream::Stdout);
        match event {
            AgentEvent::Message { content } => assert_eq!(content, "Hello world"),
            _ => panic!("Expected Message event"),
        }
    }
}
