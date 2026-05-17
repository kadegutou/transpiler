# C++ ↔ Rust Transpiler

A bidirectional source-to-source transpiler between C++17/20 and Rust.

[中文](#chinese) | English

## Overview

A three-phase compiler that converts source code between C++ and Rust while preserving semantics. The target use case is single-file, no-external-dependency algorithm and data structure code (e.g., LeetCode-style).

- **C++ → Rust**: Parses modern C++ via `tree-sitter-cpp`, emits idiomatic Rust.
- **Rust → C++**: Planned — parser stub in place (intended to use `syn`).

Unsupported constructs produce `// TODO: manual review needed` comments rather than silently dropping code.

## Architecture

```
Source → Parser → IR (language-neutral AST) → CodeGen → Target
```

| Layer | C++ Side | Rust Side |
|-------|----------|-----------|
| Parser | `tree-sitter-cpp` | stub (`syn` planned) |
| IR | `src/ir.rs` — unified AST with ownership, lifetimes, control flow | |
| CodeGen | `src/codegen/cpp.rs` → C++20 | `src/codegen/rust.rs` → Rust |

## Quick Start

```bash
# Build
cargo build --release

# C++ → Rust
cargo run -- --from cpp --to rust input.cpp -o output.rs

# Rust → C++ (parser is a stub — see above)
cargo run -- --from rust --to cpp input.rs -o output.cpp

# GUI mode (no CLI args)
cargo run

# Run tests
cargo test
```

## Semantic Mappings

| C++ | Rust |
|-----|------|
| `std::unique_ptr<T>` | `Box<T>` |
| `std::shared_ptr<T>` | `Rc<T>` / `Arc<T>` |
| `std::vector<T>` | `Vec<T>` |
| `std::string` | `String` |
| `std::optional<T>` | `Option<T>` |
| `const T&` / `T&` | `&T` / `&mut T` |
| `class` / `struct` | `struct` + `impl` |
| `new` / `delete` | `Box::new` / Drop |
| `virtual` | `trait` + `dyn` (TODO) |
| `std::cout << x << std::endl` | `println!("{}", x)` |

## License

MIT OR Apache-2.0

---

<a id="chinese"></a>

# C++ ↔ Rust 转译器

一个 C++17/20 与 Rust 之间的双向源码到源码转译工具。

## 概述

经典三段式编译器架构，支持 C++ 与 Rust 代码的语义级互转。目标场景为单文件、无外部依赖的算法与数据结构代码（如 LeetCode 风格）。

- **C++ → Rust**：通过 `tree-sitter-cpp` 解析现代 C++，生成地道的 Rust 代码。
- **Rust → C++**：规划中——解析器目前为桩代码（计划使用 `syn`）。

不支持的语法会生成 `// TODO: manual review needed` 注释，而非静默丢弃。

## 架构

```
源码 → Parser（解析器）→ IR（语言无关中间表示）→ CodeGen（代码生成）→ 目标代码
```

| 层 | C++ 侧 | Rust 侧 |
|----|--------|---------|
| 解析器 | `tree-sitter-cpp` | 桩代码（计划用 `syn`） |
| IR | `src/ir.rs` — 统一 AST，建模所有权、生命周期、控制流 | |
| 代码生成 | `src/codegen/cpp.rs` → C++20 | `src/codegen/rust.rs` → Rust |

## 快速开始

```bash
# 构建
cargo build --release

# C++ → Rust
cargo run -- --from cpp --to rust input.cpp -o output.rs

# Rust → C++（解析器为桩代码，见上）
cargo run -- --from rust --to cpp input.rs -o output.cpp

# GUI 模式（无命令行参数）
cargo run

# 运行测试
cargo test
```

## 语义映射

| C++ | Rust |
|-----|------|
| `std::unique_ptr<T>` | `Box<T>` |
| `std::shared_ptr<T>` | `Rc<T>` / `Arc<T>` |
| `std::vector<T>` | `Vec<T>` |
| `std::string` | `String` |
| `std::optional<T>` | `Option<T>` |
| `const T&` / `T&` | `&T` / `&mut T` |
| `class` / `struct` | `struct` + `impl` |
| `new` / `delete` | `Box::new` / Drop |
| `virtual` | `trait` + `dyn` (TODO) |
| `std::cout << x << std::endl` | `println!("{}", x)` |

## 许可证

MIT OR Apache-2.0
