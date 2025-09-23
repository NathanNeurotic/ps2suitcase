//! Experimental utilities for exploring the PS2 memory-card FAT.
//!
//! The code in this crate is an early-stage refactor that currently powers only
//! internal experiments and example binaries. It intentionally stays in the
//! workspace so the APIs can iterate alongside the rest of the toolchain
//! without being published to crates.io.

pub mod dir_entry;
pub mod fat;

pub use fat::Memcard;
