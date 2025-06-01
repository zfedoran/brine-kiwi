use thiserror::Error;

#[derive(Debug, Error)]
pub enum KiwiError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error at line {line}, column {column}: {msg}")]
    ParseError {
        msg:    String,
        line:   usize,
        column: usize,
    },

    #[error("Invalid enum variant \"{0}\"")]
    InvalidEnumVariant(String),

    #[error("Missing required field \"{0}\"")]
    MissingField(String),

    #[error("Schema decode error: {0}")]
    DecodeError(String),

    #[error("Schema encode error: {0}")]
    EncodeError(String),

    #[error("Verifier error: {0}")]
    VerifierError(String),
}
