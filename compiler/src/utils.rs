use crate::error::KiwiError;
use serde_json;

/// Quote a string as JSON (so that things like newlines, quotes, etc. are escaped).
pub fn quote(text: &str) -> String {
    serde_json::to_string(text).unwrap()
}

/// Return a KiwiError::ParseError.
/// Callers should do something like `return Err(error("msg", line, col));`
pub fn error(msg: &str, line: usize, column: usize) -> KiwiError {
    KiwiError::ParseError {
        msg: msg.to_string(),
        line,
        column,
    }
}
