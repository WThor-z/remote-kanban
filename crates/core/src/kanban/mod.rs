//! Kanban board management
//!
//! This module provides kanban board state management with support for
//! the three-column layout (Todo, Doing, Done) used by the frontend.

mod model;
mod store;

pub use model::*;
pub use store::*;
