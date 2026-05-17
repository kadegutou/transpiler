//! # C++ ↔ Rust Transpiler
//!
//! A bidirectional source-to-source transpiler supporting modern C++ (17/20)
//! and Rust.
//!
//! ## Architecture
//! - **Parser**: Frontends for C++ (libclang) and Rust (syn).
//! - **IR**: Language-independent AST capturing ownership, lifetimes, and control flow.
//! - **CodeGen**: Target-language code generators applying idiomatic patterns.

pub mod codegen;
pub mod ir;
pub mod parser;

use anyhow::Result;

/// Transpilation direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    CppToRust,
    RustToCpp,
}

/// Transpile a single source file from one language to another.
pub fn transpile(source: &str, direction: Direction) -> Result<String> {
    let ir = match direction {
        Direction::CppToRust => parser::cpp::parse(source)?,
        Direction::RustToCpp => parser::rust::parse(source)?,
    };

    let output = match direction {
        Direction::CppToRust => codegen::rust::generate(&ir)?,
        Direction::RustToCpp => codegen::cpp::generate(&ir)?,
    };

    Ok(output)
}
