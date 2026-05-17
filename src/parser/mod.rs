//! # Parsers
//!
//! Frontends that consume source code and produce the IR.
//!
//! ## Planned implementations
//! - **C++**: `libclang` or `tree-sitter-cpp`.
//! - **Rust**: `syn` crate.

pub mod cpp;
pub mod rust;
