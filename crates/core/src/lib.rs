//! Core library for OpenCode Vibe Kanban
//!
//! This crate contains the core business logic, including:
//! - Task management
//! - Kanban board management
//! - Project management
//! - Agent configuration

pub mod error;
pub mod kanban;
pub mod task;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
