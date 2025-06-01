use crate::{
    TYPE_INT, TYPE_UINT, TYPE_FLOAT, TYPE_STRING, TYPE_INT64, TYPE_UINT64, TYPE_BOOL, TYPE_BYTE, 
    bb::{ ByteBuffer, ByteBufferMut}, 
    schema::{DefKind, Field, Schema},
};

use std::collections::HashMap;
use std::f32;
use std::fmt;
use std::ops::Index;
use std::str;

/// This type holds dynamic Kiwi data.
///
/// Values can represent anything in a Kiwi schema and can be converted to and
/// from byte arrays using the corresponding [Schema](struct.Schema.html).
/// Enums and field names are stored using string slices from their Schema
/// for efficiency. This means that a Value can outlive the buffer it was parsed
/// from but can't outlive the schema.
#[derive(Clone, PartialEq)]
pub enum Value<'a> {
    Bool(bool),
    Byte(u8),
    Int(i32),
    UInt(u32),
    Float(f32),
    String(String),
    Int64(i64),
    UInt64(u64),
    Array(Vec<Value<'a>>),
    Enum(&'a str, &'a str),
    Object(&'a str, HashMap<&'a str, Value<'a>>),
}

impl<'a> Value<'a> {
    /// A convenience method to extract the value out of a [Bool](#variant.Bool).
    /// Returns `false` for other value kinds.
    pub fn as_bool(&self) -> bool {
        match *self {
            Value::Bool(value) => value,
            _ => false,
        }
    }

    /// A convenience method to extract the value out of a [Byte](#variant.Byte).
    /// Returns `0` for other value kinds.
    pub fn as_byte(&self) -> u8 {
        match *self {
            Value::Byte(value) => value,
            _ => 0,
        }
    }

    /// A convenience method to extract the value out of an [Int](#variant.Int).
    /// Returns `0` for other value kinds.
    pub fn as_int(&self) -> i32 {
        match *self {
            Value::Int(value) => value,
            _ => 0,
        }
    }

    /// A convenience method to extract the value out of a [UInt](#variant.UInt).
    /// Returns `0` for other value kinds.
    pub fn as_uint(&self) -> u32 {
        match *self {
            Value::UInt(value) => value,
            _ => 0,
        }
    }

    /// A convenience method to extract the value out of a [UInt64](#variant.UInt64).
    /// Returns `0` for other value kinds.
    pub fn as_int64(&self) -> i64 {
        match *self {
            Value::Int64(value) => value,
            _ => 0,
        }
    }

    /// A convenience method to extract the value out of a [UInt64](#variant.UInt64).
    /// Returns `0` for other value kinds.
    pub fn as_uint64(&self) -> u64 {
        match *self {
            Value::UInt64(value) => value,
            _ => 0,
        }
    }

    /// A convenience method to extract the value out of a [Float](#variant.Float).
    /// Returns `0.0` for other value kinds.
    pub fn as_float(&self) -> f32 {
        match *self {
            Value::Float(value) => value,
            _ => 0.0,
        }
    }

    /// A convenience method to extract the value out of a [String](#variant.String).
    /// Returns `""` for other value kinds.
    pub fn as_string(&self) -> &str {
        match *self {
            Value::String(ref value) => value.as_str(),
            Value::Enum(_, value) => value,
            _ => "",
        }
    }

    /// A convenience method to get an array of values out of an [Array](#variant.Array).
    /// Returns an empty array for other value kinds.
    pub fn as_array(&self) -> &[Value<'a>] {
        match *self {
            Value::Array(ref values) => values.as_slice(),
            _ => &[],
        }
    }

    /// A convenience method to extract the value out of an [Enum](#variant.Enum).
    /// Returns `("", "")` for other value kinds.
    pub fn as_enum(&self) -> (&str, &str) {
        match *self {
            Value::Enum(name, value) => (name, value),
            _ => ("", ""),
        }
    }

    /// A convenience method to extract the length out of an [Array](#variant.Array).
    /// Returns `0` for other value kinds.
    pub fn len(&self) -> usize {
        match *self {
            Value::Array(ref values) => values.len(),
            _ => 0,
        }
    }

    /// A convenience method to append to an [Array](#variant.Array). Does
    /// nothing for other value kinds.
    pub fn push(&mut self, value: Value<'a>) {
        if let Value::Array(ref mut values) = *self {
            values.push(value);
        }
    }

    /// A convenience method to extract a field out of an [Object](#variant.Object).
    /// Returns `None` for other value kinds or if the field isn't present.
    pub fn get(&self, name: &str) -> Option<&Value<'a>> {
        match *self {
            Value::Object(_, ref fields) => fields.get(name),
            _ => None,
        }
    }

    /// A convenience method to update a field on an [Object](#variant.Object).
    /// Does nothing for other value kinds.
    pub fn set(&mut self, name: &'a str, value: Value<'a>) {
        if let Value::Object(_, ref mut fields) = *self {
            fields.insert(name, value);
        }
    }

    /// A convenience method to remove a field on an [Object](#variant.Object).
    /// Does nothing for other value kinds.
    pub fn remove(&mut self, name: &'a str) {
        if let Value::Object(_, ref mut fields) = *self {
            fields.remove(name);
        }
    }

    /// Decodes the type specified by `type_id` and `schema` from `bytes`.
    pub fn decode(schema: &'a Schema, type_id: i32, bytes: &[u8]) -> Result<Value<'a>, ()> {
        Value::decode_bb(schema, type_id, &mut ByteBuffer::new(bytes))
    }

    /// Encodes this value into an array of bytes using the provided `schema`.
    pub fn encode(&self, schema: &Schema) -> Vec<u8> {
        let mut bb = ByteBufferMut::new();
        self.encode_bb(schema, &mut bb);
        bb.data()
    }

    /// Decodes the type specified by `type_id` and `schema` from `bb` starting
    /// at the current index. After this function returns, the current index will
    /// be advanced by the amount of data that was successfully parsed. This is
    /// mainly useful as a helper routine for [decode](#method.decode), which you
    /// probably want to use instead.
    pub fn decode_bb(
        schema: &'a Schema,
        type_id: i32,
        bb: &mut ByteBuffer,
    ) -> Result<Value<'a>, ()> {
        match type_id {
            TYPE_BOOL => Ok(Value::Bool(bb.read_bool()?)),
            TYPE_BYTE => Ok(Value::Byte(bb.read_byte()?)),
            TYPE_INT => Ok(Value::Int(bb.read_var_int()?)),
            TYPE_UINT => Ok(Value::UInt(bb.read_var_uint()?)),
            TYPE_FLOAT => Ok(Value::Float(bb.read_var_float()?)),
            TYPE_STRING => Ok(Value::String(bb.read_string()?.into_owned())),
            TYPE_INT64 => Ok(Value::Int64(bb.read_var_int64()?)),
            TYPE_UINT64 => Ok(Value::UInt64(bb.read_var_uint64()?)),

            _ => {
                let def = &schema.defs[type_id as usize];

                match def.kind {
                    DefKind::Enum => {
                        if let Some(index) = def.field_value_to_index.get(&bb.read_var_uint()?) {
                            Ok(Value::Enum(
                                def.name.as_str(),
                                def.fields[*index].name.as_str(),
                            ))
                        } else {
                            Err(())
                        }
                    }

                    DefKind::Struct => {
                        let mut fields = HashMap::new();
                        for field in &def.fields {
                            fields.insert(
                                field.name.as_str(),
                                Value::decode_field_bb(schema, field, bb)?,
                            );
                        }
                        Ok(Value::Object(def.name.as_str(), fields))
                    }

                    DefKind::Message => {
                        let mut fields = HashMap::new();
                        loop {
                            let value = bb.read_var_uint()?;
                            if value == 0 {
                                return Ok(Value::Object(def.name.as_str(), fields));
                            }
                            if let Some(index) = def.field_value_to_index.get(&value) {
                                let field = &def.fields[*index];
                                fields.insert(
                                    field.name.as_str(),
                                    Value::decode_field_bb(schema, field, bb)?,
                                );
                            } else {
                                return Err(());
                            }
                        }
                    }
                }
            }
        }
    }

    /// Decodes the field specified by `field` and `schema` from `bb` starting
    /// at the current index. This is used by [decode_bb](#method.decode_bb) but
    /// may also be useful by itself.
    pub fn decode_field_bb(
        schema: &'a Schema,
        field: &Field,
        bb: &mut ByteBuffer,
    ) -> Result<Value<'a>, ()> {
        if field.is_array {
            let len = bb.read_var_uint()? as usize;
            let mut array = Vec::with_capacity(len);
            for _ in 0..len {
                array.push(Value::decode_bb(schema, field.type_id, bb)?);
            }
            Ok(Value::Array(array))
        } else {
            Value::decode_bb(schema, field.type_id, bb)
        }
    }

    /// Encodes the current value to the end of `bb` using the provided `schema`.
    /// This is mainly useful as a helper routine for [encode](#method.encode),
    /// which you probably want to use instead.
    pub fn encode_bb(&self, schema: &Schema, bb: &mut ByteBufferMut) {
        match *self {
            Value::Bool(value) => bb.write_byte(if value { 1 } else { 0 }),
            Value::Byte(value) => bb.write_byte(value),
            Value::Int(value) => bb.write_var_int(value),
            Value::UInt(value) => bb.write_var_uint(value),
            Value::Float(value) => bb.write_var_float(value),
            Value::String(ref value) => bb.write_string(value.as_str()),
            Value::Int64(value) => bb.write_var_int64(value),
            Value::UInt64(value) => bb.write_var_uint64(value),

            Value::Array(ref values) => {
                bb.write_var_uint(values.len() as u32);
                for value in values {
                    value.encode_bb(schema, bb);
                }
                return;
            }

            Value::Enum(name, value) => {
                let def = &schema.defs[*schema.def_name_to_index.get(name).unwrap()];
                let index = *def.field_name_to_index.get(value).unwrap();
                bb.write_var_uint(def.fields[index].value);
            }

            Value::Object(name, ref fields) => {
                let def = &schema.defs[*schema.def_name_to_index.get(name).unwrap()];
                match def.kind {
                    DefKind::Enum => panic!(),
                    DefKind::Struct => {
                        for field in &def.fields {
                            fields
                                .get(field.name.as_str())
                                .unwrap()
                                .encode_bb(schema, bb);
                        }
                    }
                    DefKind::Message => {
                        // Loop over all fields to ensure consistent encoding order
                        for field in &def.fields {
                            if let Some(value) = fields.get(field.name.as_str()) {
                                bb.write_var_uint(field.value);
                                value.encode_bb(schema, bb);
                            }
                        }
                        bb.write_byte(0);
                    }
                }
            }
        }
    }
}

impl<'a> Index<usize> for Value<'a> {
    type Output = Value<'a>;

    /// A convenience method that adds support for `self[index]` expressions.
    /// It will panic if this value isn't an [Array](#variant.Array) or if the
    /// provided index is out of bounds.
    fn index(&self, index: usize) -> &Value<'a> {
        match *self {
            Value::Array(ref values) => &values[index],
            _ => panic!(),
        }
    }
}

impl<'a> fmt::Debug for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Value::Bool(value) => value.fmt(f),
            Value::Byte(value) => value.fmt(f),
            Value::Int(value) => value.fmt(f),
            Value::UInt(value) => value.fmt(f),
            Value::Float(value) => value.fmt(f),
            Value::String(ref value) => value.fmt(f),
            Value::Int64(value) => value.fmt(f),
            Value::UInt64(value) => value.fmt(f),
            Value::Array(ref values) => values.fmt(f),
            Value::Enum(name, ref value) => write!(f, "{}::{}", name, value),

            Value::Object(name, ref fields) => {
                let mut keys: Vec<_> = fields.keys().collect();
                let mut first = true;
                keys.sort();
                write!(f, "{} {{", name)?;

                for key in keys {
                    if first {
                        first = false;
                    } else {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {:?}", key, fields[key])?;
                }

                write!(f, "}}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Def, Field};

    #[test]
    fn value_basic() {
        let value = Value::Array(vec![
            Value::Bool(true),
            Value::Byte(255),
            Value::Int(-1),
            Value::UInt(1),
            Value::Float(0.5),
            Value::String("abc".to_owned()),
            Value::Enum("Foo", "FOO"),
            Value::Object("Obj", {
                let mut map = HashMap::new();
                map.insert("key1", Value::String("value1".to_owned()));
                map.insert("key2", Value::String("value2".to_owned()));
                map
            }),
        ]);

        assert_eq!(value.len(), 8);

        assert_eq!(value[0], Value::Bool(true));
        assert_eq!(value[1], Value::Byte(255));
        assert_eq!(value[2], Value::Int(-1));
        assert_eq!(value[3], Value::UInt(1));
        assert_eq!(value[4], Value::Float(0.5));
        assert_eq!(value[5], Value::String("abc".to_owned()));
        assert_eq!(value[6], Value::Enum("Foo", "FOO"));
        assert_eq!(
            value[7],
            Value::Object("Obj", {
                let mut map = HashMap::new();
                map.insert("key1", Value::String("value1".to_owned()));
                map.insert("key2", Value::String("value2".to_owned()));
                map
            })
        );

        assert_eq!(value[0].as_bool(), true);
        assert_eq!(value[1].as_byte(), 255);
        assert_eq!(value[2].as_int(), -1);
        assert_eq!(value[3].as_uint(), 1);
        assert_eq!(value[4].as_float(), 0.5);
        assert_eq!(value[5].as_string(), "abc");
        assert_eq!(value.get("key1"), None);
        assert_eq!(
            value[7].get("key1"),
            Some(&Value::String("value1".to_owned()))
        );

        assert_eq!(
            format!("{:?}", value),
            "[true, 255, -1, 1, 0.5, \"abc\", Foo::FOO, Obj {key1: \"value1\", key2: \"value2\"}]"
        );
    }

    #[test]
    fn value_push() {
        let mut value = Value::Array(vec![]);
        assert_eq!(value.len(), 0);

        value.push(Value::Int(123));
        assert_eq!(value.len(), 1);
        assert_eq!(value[0], Value::Int(123));

        value.push(Value::Int(456));
        assert_eq!(value.len(), 2);
        assert_eq!(value[0], Value::Int(123));
        assert_eq!(value[1], Value::Int(456));
    }

    #[test]
    fn value_set() {
        let mut value = Value::Object("Foo", HashMap::new());
        assert_eq!(value.get("x"), None);

        value.set("x", Value::Int(123));
        assert_eq!(value.get("x"), Some(&Value::Int(123)));

        value.set("y", Value::Int(456));
        assert_eq!(value.get("x"), Some(&Value::Int(123)));
        assert_eq!(value.get("y"), Some(&Value::Int(456)));

        value.set("x", Value::Int(789));
        assert_eq!(value.get("x"), Some(&Value::Int(789)));
        assert_eq!(value.get("y"), Some(&Value::Int(456)));
    }

    #[test]
    fn value_remove() {
        let mut value = Value::Object("Foo", HashMap::new());
        assert_eq!(value.get("x"), None);

        value.set("x", Value::Int(123));
        assert_eq!(value.get("x"), Some(&Value::Int(123)));

        value.set("y", Value::Int(456));
        assert_eq!(value.get("x"), Some(&Value::Int(123)));
        assert_eq!(value.get("y"), Some(&Value::Int(456)));

        value.remove("x");
        assert_eq!(value.get("x"), None);
        assert_eq!(value.get("y"), Some(&Value::Int(456)));

        value.remove("y");
        assert_eq!(value.get("x"), None);
        assert_eq!(value.get("y"), None);
    }

    #[test]
    fn value_encode_and_decode() {
        let schema = Schema::new(vec![
            Def::new(
                "Enum".to_owned(),
                DefKind::Enum,
                vec![
                    Field {
                        name: "FOO".to_owned(),
                        type_id: 0,
                        is_array: false,
                        value: 100,
                    },
                    Field {
                        name: "BAR".to_owned(),
                        type_id: 0,
                        is_array: false,
                        value: 200,
                    },
                ],
            ),
            Def::new(
                "Struct".to_owned(),
                DefKind::Struct,
                vec![
                    Field {
                        name: "v_enum".to_owned(),
                        type_id: 0,
                        is_array: true,
                        value: 0,
                    },
                    Field {
                        name: "v_message".to_owned(),
                        type_id: 2,
                        is_array: false,
                        value: 0,
                    },
                ],
            ),
            Def::new(
                "Message".to_owned(),
                DefKind::Message,
                vec![
                    Field {
                        name: "v_bool".to_owned(),
                        type_id: TYPE_BOOL,
                        is_array: false,
                        value: 1,
                    },
                    Field {
                        name: "v_byte".to_owned(),
                        type_id: TYPE_BYTE,
                        is_array: false,
                        value: 2,
                    },
                    Field {
                        name: "v_int".to_owned(),
                        type_id: TYPE_INT,
                        is_array: false,
                        value: 3,
                    },
                    Field {
                        name: "v_uint".to_owned(),
                        type_id: TYPE_UINT,
                        is_array: false,
                        value: 4,
                    },
                    Field {
                        name: "v_float".to_owned(),
                        type_id: TYPE_FLOAT,
                        is_array: false,
                        value: 5,
                    },
                    Field {
                        name: "v_string".to_owned(),
                        type_id: TYPE_STRING,
                        is_array: false,
                        value: 6,
                    },
                    Field {
                        name: "v_int64".to_owned(),
                        type_id: TYPE_INT64,
                        is_array: false,
                        value: 7,
                    },
                    Field {
                        name: "v_uint64".to_owned(),
                        type_id: TYPE_UINT64,
                        is_array: false,
                        value: 8,
                    },
                    Field {
                        name: "v_enum".to_owned(),
                        type_id: 0,
                        is_array: false,
                        value: 9,
                    },
                    Field {
                        name: "v_struct".to_owned(),
                        type_id: 1,
                        is_array: false,
                        value: 10,
                    },
                    Field {
                        name: "v_message".to_owned(),
                        type_id: 2,
                        is_array: false,
                        value: 11,
                    },
                    Field {
                        name: "a_bool".to_owned(),
                        type_id: TYPE_BOOL,
                        is_array: true,
                        value: 12,
                    },
                    Field {
                        name: "a_byte".to_owned(),
                        type_id: TYPE_BYTE,
                        is_array: true,
                        value: 13,
                    },
                    Field {
                        name: "a_int".to_owned(),
                        type_id: TYPE_INT,
                        is_array: true,
                        value: 14,
                    },
                    Field {
                        name: "a_uint".to_owned(),
                        type_id: TYPE_UINT,
                        is_array: true,
                        value: 15,
                    },
                    Field {
                        name: "a_float".to_owned(),
                        type_id: TYPE_FLOAT,
                        is_array: true,
                        value: 16,
                    },
                    Field {
                        name: "a_string".to_owned(),
                        type_id: TYPE_STRING,
                        is_array: true,
                        value: 17,
                    },
                    Field {
                        name: "a_int64".to_owned(),
                        type_id: TYPE_INT64,
                        is_array: true,
                        value: 18,
                    },
                    Field {
                        name: "a_uint64".to_owned(),
                        type_id: TYPE_UINT64,
                        is_array: true,
                        value: 19,
                    },
                    Field {
                        name: "a_enum".to_owned(),
                        type_id: 0,
                        is_array: true,
                        value: 20,
                    },
                    Field {
                        name: "a_struct".to_owned(),
                        type_id: 1,
                        is_array: true,
                        value: 21,
                    },
                    Field {
                        name: "a_message".to_owned(),
                        type_id: 2,
                        is_array: true,
                        value: 22,
                    },
                ],
            ),
        ]);

        assert!(Schema::decode(&schema.encode()).is_ok());

        assert_eq!(
            Value::decode(&schema, TYPE_BOOL, &[0]),
            Ok(Value::Bool(false))
        );
        assert_eq!(
            Value::decode(&schema, TYPE_BOOL, &[1]),
            Ok(Value::Bool(true))
        );
        assert_eq!(Value::decode(&schema, TYPE_BOOL, &[2]), Err(()));
        assert_eq!(
            Value::decode(&schema, TYPE_BYTE, &[255]),
            Ok(Value::Byte(255))
        );
        assert_eq!(Value::decode(&schema, TYPE_INT, &[1]), Ok(Value::Int(-1)));
        assert_eq!(Value::decode(&schema, TYPE_UINT, &[1]), Ok(Value::UInt(1)));
        assert_eq!(
            Value::decode(&schema, TYPE_FLOAT, &[126, 0, 0, 0]),
            Ok(Value::Float(0.5))
        );
        assert_eq!(
            Value::decode(&schema, TYPE_STRING, &[240, 159, 141, 149, 0]),
            Ok(Value::String("🍕".to_owned()))
        );
        assert_eq!(
            Value::decode(&schema, TYPE_INT64, &[1]),
            Ok(Value::Int64(-1))
        );
        assert_eq!(
            Value::decode(&schema, TYPE_UINT64, &[1]),
            Ok(Value::UInt64(1))
        );
        assert_eq!(Value::decode(&schema, 0, &[0]), Err(()));
        assert_eq!(
            Value::decode(&schema, 0, &[100]),
            Ok(Value::Enum("Enum", "FOO"))
        );
        assert_eq!(
            Value::decode(&schema, 0, &[200, 1]),
            Ok(Value::Enum("Enum", "BAR"))
        );

        assert_eq!(Value::Bool(false).encode(&schema), [0]);
        assert_eq!(Value::Bool(true).encode(&schema), [1]);
        assert_eq!(Value::Byte(255).encode(&schema), [255]);
        assert_eq!(Value::Int(-1).encode(&schema), [1]);
        assert_eq!(Value::UInt(1).encode(&schema), [1]);
        assert_eq!(Value::Float(0.5).encode(&schema), [126, 0, 0, 0]);
        assert_eq!(
            Value::String("🍕".to_owned()).encode(&schema),
            [240, 159, 141, 149, 0]
        );
        assert_eq!(Value::Int64(-1).encode(&schema), [1]);
        assert_eq!(Value::UInt64(1).encode(&schema), [1]);
        assert_eq!(Value::Enum("Enum", "FOO").encode(&schema), [100]);
        assert_eq!(Value::Enum("Enum", "BAR").encode(&schema), [200, 1]);

        fn insert<'a>(
            mut map: HashMap<&'a str, Value<'a>>,
            key: &'a str,
            value: Value<'a>,
        ) -> HashMap<&'a str, Value<'a>> {
            map.insert(key, value);
            map
        }

        let empty_struct = Value::Object(
            "Struct",
            insert(
                insert(HashMap::new(), "v_enum", Value::Array(vec![])),
                "v_message",
                Value::Object("Message", HashMap::new()),
            ),
        );

        assert_eq!(Value::decode(&schema, 1, &[0, 0]), Ok(empty_struct.clone()));
        assert_eq!(empty_struct.encode(&schema), [0, 0]);

        let full_struct = Value::Object(
            "Struct",
            insert(
                insert(
                    HashMap::new(),
                    "v_enum",
                    Value::Array(vec![Value::Enum("Enum", "FOO"), Value::Enum("Enum", "BAR")]),
                ),
                "v_message",
                Value::Object(
                    "Message",
                    insert(HashMap::new(), "v_string", Value::String("🍕".to_owned())),
                ),
            ),
        );

        assert_eq!(
            Value::decode(&schema, 1, &[2, 100, 200, 1, 6, 240, 159, 141, 149, 0, 0]),
            Ok(full_struct.clone())
        );
        assert_eq!(
            full_struct.encode(&schema),
            [2, 100, 200, 1, 6, 240, 159, 141, 149, 0, 0]
        );

        assert_eq!(
            Value::Object(
                "Message",
                insert(HashMap::new(), "v_bool", Value::Bool(false))
            )
            .encode(&schema),
            [1, 0, 0]
        );
        assert_eq!(
            Value::Object(
                "Message",
                insert(HashMap::new(), "v_bool", Value::Bool(true))
            )
            .encode(&schema),
            [1, 1, 0]
        );
        assert_eq!(
            Value::Object(
                "Message",
                insert(HashMap::new(), "v_byte", Value::Byte(255))
            )
            .encode(&schema),
            [2, 255, 0]
        );
        assert_eq!(
            Value::Object("Message", insert(HashMap::new(), "v_int", Value::Int(-1))).encode(&schema),
            [3, 1, 0]
        );
        assert_eq!(
            Value::Object("Message", insert(HashMap::new(), "v_uint", Value::UInt(1))).encode(&schema),
            [4, 1, 0]
        );
        assert_eq!(
            Value::Object(
                "Message",
                insert(HashMap::new(), "v_float", Value::Float(0.0))
            )
            .encode(&schema),
            [5, 0, 0]
        );
        assert_eq!(
            Value::Object(
                "Message",
                insert(HashMap::new(), "v_string", Value::String("".to_owned()))
            )
            .encode(&schema),
            [6, 0, 0]
        );
        assert_eq!(
            Value::Object("Message", insert(HashMap::new(), "v_int64", Value::Int(-1))).encode(&schema),
            [7, 1, 0]
        );
        assert_eq!(
            Value::Object(
                "Message",
                insert(HashMap::new(), "v_uint64", Value::UInt(1))
            )
            .encode(&schema),
            [8, 1, 0]
        );
        assert_eq!(
            Value::Object(
                "Message",
                insert(HashMap::new(), "v_enum", Value::Enum("Enum", "FOO"))
            )
            .encode(&schema),
            [9, 100, 0]
        );
        assert_eq!(
            Value::Object(
                "Message",
                insert(HashMap::new(), "v_struct", empty_struct.clone())
            )
            .encode(&schema),
            [10, 0, 0, 0]
        );
        assert_eq!(
            Value::Object(
                "Message",
                insert(
                    HashMap::new(),
                    "v_message",
                    Value::Object("Message", HashMap::new())
                )
            )
            .encode(&schema),
            [11, 0, 0]
        );

        assert_eq!(
            Value::decode(&schema, 2, &[1, 0, 0]),
            Ok(Value::Object(
                "Message",
                insert(HashMap::new(), "v_bool", Value::Bool(false))
            ))
        );
        assert_eq!(
            Value::decode(&schema, 2, &[1, 1, 0]),
            Ok(Value::Object(
                "Message",
                insert(HashMap::new(), "v_bool", Value::Bool(true))
            ))
        );
        assert_eq!(
            Value::decode(&schema, 2, &[2, 255, 0]),
            Ok(Value::Object(
                "Message",
                insert(HashMap::new(), "v_byte", Value::Byte(255))
            ))
        );
        assert_eq!(
            Value::decode(&schema, 2, &[3, 1, 0]),
            Ok(Value::Object(
                "Message",
                insert(HashMap::new(), "v_int", Value::Int(-1))
            ))
        );
        assert_eq!(
            Value::decode(&schema, 2, &[4, 1, 0]),
            Ok(Value::Object(
                "Message",
                insert(HashMap::new(), "v_uint", Value::UInt(1))
            ))
        );
        assert_eq!(
            Value::decode(&schema, 2, &[5, 0, 0]),
            Ok(Value::Object(
                "Message",
                insert(HashMap::new(), "v_float", Value::Float(0.0))
            ))
        );
        assert_eq!(
            Value::decode(&schema, 2, &[6, 0, 0]),
            Ok(Value::Object(
                "Message",
                insert(HashMap::new(), "v_string", Value::String("".to_owned()))
            ))
        );
        assert_eq!(
            Value::decode(&schema, 2, &[7, 1, 0]),
            Ok(Value::Object(
                "Message",
                insert(HashMap::new(), "v_int64", Value::Int64(-1))
            ))
        );
        assert_eq!(
            Value::decode(&schema, 2, &[8, 1, 0]),
            Ok(Value::Object(
                "Message",
                insert(HashMap::new(), "v_uint64", Value::UInt64(1))
            ))
        );
        assert_eq!(
            Value::decode(&schema, 2, &[9, 100, 0]),
            Ok(Value::Object(
                "Message",
                insert(HashMap::new(), "v_enum", Value::Enum("Enum", "FOO"))
            ))
        );
        assert_eq!(
            Value::decode(&schema, 2, &[10, 0, 0, 0]),
            Ok(Value::Object(
                "Message",
                insert(HashMap::new(), "v_struct", empty_struct.clone())
            ))
        );
        assert_eq!(
            Value::decode(&schema, 2, &[11, 0, 0]),
            Ok(Value::Object(
                "Message",
                insert(
                    HashMap::new(),
                    "v_message",
                    Value::Object("Message", HashMap::new())
                )
            ))
        );
    }

    // This test case is for a bug where rustc was silently inferring an incorrect
    // lifetime. This is the specific error:
    //
    //   error[E0597]: `value` does not live long enough
    //       --> src/lib.rs:1307:40
    //        |
    //   1307 |     if let Some(Value::Array(items)) = value.get("items") {
    //        |                                        ^^^^^ borrowed value does not live long enough
    //   ...
    //   1312 |   }
    //        |   - borrowed value only lives until here
    //        |
    //        = note: borrowed value must be valid for the static lifetime...
    //
    // The fix was to change this:
    //
    //   pub fn get(&self, name: &str) -> Option<&Value> {
    //
    // Into this:
    //
    //   pub fn get(&self, name: &str) -> Option<&Value<'a>> {
    //
    #[test]
    fn value_get_bad_lifetime_inference_in_rustc() {
        fn use_item<'a>(_: &'a Value<'static>) {}

        fn use_items(value: Value<'static>) {
            if let Some(Value::Array(items)) = value.get("items") {
                for item in items {
                    use_item(item);
                }
            }
        }

        use_items(Value::Array(vec![]));
    }
}
