//! brine-kiwi-sdk
//!
//! This crate provides runtime support for working with Kiwi-encoded data.
//! 
//! - `FromKiwi` trait (re-exported from compiler)  
//! - Helpers for reading/writing flat binary streams, etc.

pub use brine_kiwi_compiler::traits::FromKiwi;
pub use brine_kiwi_compiler::error::KiwiError;
pub use brine_kiwi_schema::{ Schema, Field, Value };

/// Decode a Kiwi buffer into a pretty‐printed JSON string.
pub fn decode_to_json(buffer: &[u8]) -> Result<String, KiwiError> {
    let schema = brine_kiwi_compiler::decode_binary_schema(buffer)?;
    Ok(serde_json::to_string_pretty(&schema).unwrap())
}

pub mod traits {
    pub use brine_kiwi_compiler::traits::FromKiwi;
}

pub mod error {
    pub use brine_kiwi_compiler::error::KiwiError;
}

pub mod schema {
    pub use brine_kiwi_schema::{Schema, Field, Value};
}
