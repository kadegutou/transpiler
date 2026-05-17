//! # Rust Parser
//!
//! Parses Rust source into the IR.
//!
//! ## Status
//! MVP: Currently a stub that demonstrates the pipeline.
//! Full implementation should use the `syn` crate.

use crate::ir::*;
use anyhow::{bail, Result};

/// Parse Rust source into the IR.
pub fn parse(source: &str) -> Result<Module> {
    // TODO: Replace with real `syn`-based parser.
    Ok(Module {
        items: vec![Item::TODOComment(format!(
            "Rust parser stub: source length = {} bytes. Full parser not yet implemented.",
            source.len()
        ))],
    })
}

/// Convert a Rust type string into an IR `Type`.
/// Very naive heuristic for demonstration.
pub fn rust_type_to_ir(ty: &str) -> Result<Type> {
    let ty = ty.trim();
    if ty == "()" {
        Ok(Type::Unit)
    } else if ty == "!" {
        Ok(Type::Never)
    } else if ty == "_" {
        Ok(Type::Infer)
    } else if ty == "String" || ty == "str" {
        Ok(Type::Named(ty.to_string(), vec![]))
    } else if ty == "i8" || ty == "i16" || ty == "i32" || ty == "i64"
        || ty == "u8" || ty == "u16" || ty == "u32" || ty == "u64"
        || ty == "f32" || ty == "f64" || ty == "bool" || ty == "char" || ty == "usize" || ty == "isize"
    {
        Ok(Type::Named(ty.to_string(), vec![]))
    } else if ty.starts_with("Vec<") && ty.ends_with(">") {
        let inner = &ty[4..ty.len() - 1];
        Ok(Type::Named("Vec".to_string(), vec![rust_type_to_ir(inner)?]))
    } else if ty.starts_with("Box<") && ty.ends_with(">") {
        let inner = &ty[4..ty.len() - 1];
        Ok(Type::Named("Box".to_string(), vec![rust_type_to_ir(inner)?]))
    } else if ty.starts_with("Rc<") && ty.ends_with(">") {
        let inner = &ty[3..ty.len() - 1];
        Ok(Type::Named("Rc".to_string(), vec![rust_type_to_ir(inner)?]))
    } else if ty.starts_with("Arc<") && ty.ends_with(">") {
        let inner = &ty[4..ty.len() - 1];
        Ok(Type::Named("Arc".to_string(), vec![rust_type_to_ir(inner)?]))
    } else if ty.starts_with("Option<") && ty.ends_with(">") {
        let inner = &ty[7..ty.len() - 1];
        Ok(Type::Named("Option".to_string(), vec![rust_type_to_ir(inner)?]))
    } else if ty.starts_with("Result<") && ty.ends_with(">") {
        let inner = &ty[7..ty.len() - 1];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() != 2 {
            bail!("Malformed Result type: {}", ty);
        }
        Ok(Type::Named(
            "Result".to_string(),
            vec![rust_type_to_ir(parts[0])?, rust_type_to_ir(parts[1])?],
        ))
    } else if ty.starts_with("&mut ") {
        let inner = &ty[5..];
        Ok(Type::Ref(Box::new(rust_type_to_ir(inner)?), Mutability::Mut))
    } else if ty.starts_with("&") {
        let inner = &ty[1..];
        Ok(Type::Ref(Box::new(rust_type_to_ir(inner)?), Mutability::Not))
    } else if ty.starts_with("*const ") {
        let inner = &ty[7..];
        Ok(Type::Ptr(Box::new(rust_type_to_ir(inner)?), Mutability::Not))
    } else if ty.starts_with("*mut ") {
        let inner = &ty[5..];
        Ok(Type::Ptr(Box::new(rust_type_to_ir(inner)?), Mutability::Mut))
    } else {
        Ok(Type::Named(ty.to_string(), vec![]))
    }
}
