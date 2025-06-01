//! This is a Rust library with some helper routines for parsing files in the
//! Kiwi serialization format. See [https://github.com/evanw/kiwi](https://github.com/evanw/kiwi)
//! for documentation about the format.
//!
//! ```
//! use brine_kiwi_schema::*;
//!
//! let schema = Schema::new(vec![
//!     Def::new("Point".to_owned(), DefKind::Struct, vec![
//!         Field {name: "x".to_owned(), type_id: TYPE_FLOAT, is_array: false, value: 0},
//!         Field {name: "y".to_owned(), type_id: TYPE_FLOAT, is_array: false, value: 0},
//!     ]),
//! ]);
//!
//! let value = Value::decode(&schema, 0, &[126, 0, 0, 0, 126, 1, 0, 0]).unwrap();
//! assert_eq!(format!("{:?}", value), "Point {x: 0.5, y: -0.5}");
//! assert_eq!(value.encode(&schema), [126, 0, 0, 0, 126, 1, 0, 0]);
//! ```

pub mod bb;
pub mod schema;
pub mod value;

pub use bb::*;
pub use schema::*;
pub use value::*;

pub const TYPE_BOOL: i32 = -1;
pub const TYPE_BYTE: i32 = -2;
pub const TYPE_INT: i32 = -3;
pub const TYPE_UINT: i32 = -4;
pub const TYPE_FLOAT: i32 = -5;
pub const TYPE_STRING: i32 = -6;
pub const TYPE_INT64: i32 = -7;
pub const TYPE_UINT64: i32 = -8;
