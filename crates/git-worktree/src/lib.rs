//! Git Worktree management library
//!
//! This crate provides functionality for managing Git worktrees,
//! enabling isolated task execution in separate working directories.

mod commands;
mod error;
mod worktree;

pub use error::{WorktreeError, Result};
pub use worktree::{Worktree, WorktreeConfig, WorktreeManager, WorktreeStatus};
