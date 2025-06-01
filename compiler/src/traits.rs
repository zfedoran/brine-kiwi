use crate::error::KiwiError;
use brine_kiwi_schema::Value;

/// All Kiwi‐derived types must return `Result<Self, KiwiError>`.
/// We require `Sized` so that `Self` can be constructed.
pub trait FromKiwi: Sized {
    fn from_kiwi(value: &Value) -> Result<Self, KiwiError>;
}

