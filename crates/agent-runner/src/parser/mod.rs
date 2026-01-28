//! Output parsers for different agent types

use crate::event::{AgentEvent, OutputStream};

pub mod opencode;

/// Trait for parsing agent output
pub trait OutputParser: Send + Sync {
    /// Parse a line of output
    ///
    /// Returns an AgentEvent if the line (or accumulated lines) form a complete event.
    /// Returns None if the line is consumed but doesn't complete an event yet,
    /// or if it should be treated as raw output (though usually we want to return RawOutput).
    fn parse(&mut self, line: &str, stream: OutputStream) -> AgentEvent;
}

/// Create a parser for the given agent type
pub fn create_parser(agent_type: crate::process::AgentType) -> Box<dyn OutputParser> {
    match agent_type {
        crate::process::AgentType::OpenCode => Box::new(opencode::OpenCodeParser::new()),
        _ => Box::new(DefaultParser),
    }
}

/// A default parser that just returns raw output
pub struct DefaultParser;

impl OutputParser for DefaultParser {
    fn parse(&mut self, line: &str, stream: OutputStream) -> AgentEvent {
        AgentEvent::RawOutput {
            stream,
            content: line.to_string(),
        }
    }
}
