//! Project module
//!
//! A Project represents a Git repository managed by a Gateway.
//! Tasks belong to Projects.

mod model;
mod store;

pub use model::*;
pub use store::*;
