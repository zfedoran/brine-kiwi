//! brine-kiwi-compiler
//!
//! This crate implements:
//!  1) A tokenizer + parser for `.kiwi` IDL files,
//!  2) A schema verifier (duplicate types, recursive structs, missing types, etc.),
//!  3) `encode_binary_schema` / `decode_binary_schema` (flat‐buffer style),
//!  4) Code generation (`compile_schema_to_rust` → `String`),
//!  5) Error types (`KiwiError`), and `FromKiwi` trait.

pub mod error;
pub mod types;
pub mod utils;
pub mod tokenizer;
pub mod parser;
pub mod verifier;
pub mod compiler;
pub mod gen_rust;
pub mod traits;

pub use compiler::compile_schema;
pub use compiler::decode_binary_schema;
pub use compiler::encode_binary_schema;
pub use gen_rust::compile_schema_to_rust;
