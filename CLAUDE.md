# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

A bidirectional C++17/20 ↔ Rust source-to-source transpiler. Target use case: single-file, no-external-dependency algorithm/data-structure code (e.g., LeetCode-style). Unsupported constructs generate `// TODO: manual review needed` placeholder comments rather than silently dropping code.

## Build & Test

```bash
# Build
cargo build

# Build release
cargo build --release

# Run all tests
cargo test

# Run a single test
cargo test -- test_name

# Run CLI
cargo run -- --from cpp --to rust input.cpp -o output.rs
cargo run -- --from rust --to cpp input.rs -o output.cpp

# Run GUI (no CLI args)
cargo run
```

## Architecture

Three-phase compiler pipeline enforced by `src/lib.rs:25-37`: **parse → IR → codegen**.

### 1. Parser (`src/parser/`)

- **cpp.rs** — Real parser using `tree-sitter-cpp`. Entry point: `parse(source) -> Result<Module>`. Also exports `cpp_type_to_ir()` for type-string conversion used in tests. Walks tree-sitter CST nodes and maps them to IR constructs. Handles: function definitions, declarations, class/enum specifiers. Stubs out templates, namespaces, using/alias.
- **rust.rs** — Stub parser. `parse()` currently returns a `TODOComment` for everything. Intended to use the `syn` crate eventually. Exports `rust_type_to_ir()` for naive type-string-to-IR conversion.

### 2. IR (`src/ir.rs`)

Language-neutral AST. Top node is `Module { items: Vec<Item> }`, where `Item` variants are: `Function`, `StructDef`, `EnumDef`, `TraitDef`, `ImplBlock`, `TypeAlias`, `Use`, `TODOComment`. The `Type` enum models ownership/mutability/lifetime distinctions (Ref/Ptr/Mutability) so codegen can apply idiomatic mappings. `Expr`, `Stmt`, and `Pattern` cover common expressions, statements, and match patterns.

### 3. Codegen (`src/codegen/`)

- **cpp.rs** — Emits C++20. Maps: `Box<T>`→`std::unique_ptr<T>`, `Rc<T>`→`std::shared_ptr<T>`, `Option<T>`→`std::optional<T>`, `Result<T,E>`→custom struct (C++20 compat), `&T`/`&mut T`→`const T&`/`T&`, `match`→if-else chain, `trait`→abstract base class, lifetimes→comments.
- **rust.rs** — Emits idiomatic Rust. Maps: `std::unique_ptr<T>`→`Box<T>`, `std::shared_ptr<T>`→`Rc<T>`, `std::optional<T>`→`Option<T>`, `const T&`/`T&`→`&T`/`&mut T`, `class`→`struct`+`impl`, `new`/`delete`→`Box::new`/Drop, `virtual`→`trait` (TODO). Special handling: strips `return 0;` from main() bodies since Rust main returns unit.

### 4. Entry points

- **src/main.rs** — CLI binary. Uses `clap`. No-arg invocation launches GUI mode (eframe/egui). With args: `--from {cpp|rust} --to {rust|cpp} <input> [-o <output>]`.
- **src/lib.rs** — Library crate root. Exposes `transpile(source, direction) -> Result<String>` and the `Direction` enum. Orchestrates parse→codegen.
- **src/gui.rs** — egui/eframe desktop app with dual-pane editor, language selectors, transpile/swap buttons.

### 5. C++ support library

`CMakeLists.txt` defines a header-only `transpiler_support` INTERFACE library. This provides runtime types (e.g., a C++20 `Result<T,E>` shim) needed by transpiled C++ output. Not needed for the Rust transpiler binary itself — it's output-side support.

## Key design decisions

- The IR is the single source of truth for semantic mappings between the two languages. Any new construct mapping should be added as an IR node first, then implemented in both parser direction and both codegen direction.
- The Rust parser is intentionally a stub; the MVP focuses on C++→Rust via the working tree-sitter-based C++ parser.
- Unsupported constructs use `Item::TODOComment` or `Expr::TODO` in the IR, which codegen renders as `// TODO: manual review needed` comments — ensures transpilation never silently loses code.
