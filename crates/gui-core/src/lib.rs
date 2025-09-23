//! Core data structures and helpers shared by the PSU packing frontends.
//!
//! This crate exposes pure data models, command enums, and validation utilities
//! that can be reused across different GUI implementations.  Downstream
//! consumers can import the [`actions`], [`commands`], [`state`], and
//! [`validation`] modules directly or rely on the re-exported items provided at
//! the crate root.

pub mod actions;
pub mod commands;
pub mod state;
pub mod validation;

pub use actions::*;
pub use commands::*;
pub use state::*;
pub use validation::*;
