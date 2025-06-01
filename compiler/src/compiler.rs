use brine_kiwi_schema::ByteBuffer;
use crate::{
    types::{DefinitionKind, Field, Schema},
    verifier::{verify_schema, NATIVE_TYPES},
    tokenizer::tokenize_schema,
    parser::parse_schema,
    error::KiwiError,
};

/// Compile a textual schema into `(Schema, Vec<u8>)`.
/// Returns `Err(KiwiError)` if tokenization/parsing/verification fails.
pub fn compile_schema(text: &str) -> Result<(Schema, Vec<u8>), KiwiError> {
    let tokens = tokenize_schema(text)?;
    let schema = parse_schema(&tokens)?;
    verify_schema(&schema)?;
    let bin = encode_binary_schema(&schema)?;
    Ok((schema, bin))
}

/// Decode a binary schema buffer back into a `Schema`.
/// Returns `Err(KiwiError)` on any read failure or invalid data.
pub fn decode_binary_schema(buffer: &[u8]) -> Result<Schema, KiwiError> {
    struct FieldTemp {
        name:           String,
        type_num:       i32,
        is_array:       bool,
        reserved_index: u32,
    }

    struct DefinitionTemp {
        name:   String,
        kind:   DefinitionKind,
        fields: Vec<FieldTemp>,
    }

    let mut bb = ByteBuffer::new(buffer);

    // Read definition count
    let definition_count = bb
        .read_var_uint()
        .map_err(|e| KiwiError::DecodeError(format!("Failed to read definition count: {:?}", e)))?;

    // Collect all definitions (temporarily)
    let mut definitions_temp: Vec<DefinitionTemp> =
        Vec::with_capacity(definition_count as usize);

    // Read each definition
    for _ in 0..definition_count {
        let definition_name = bb
            .read_string()
            .map_err(|e| {
                KiwiError::DecodeError(format!("Failed to read definition name: {:?}", e))
            })?
            .into_owned();

        let kind_byte = bb
            .read_byte()
            .map_err(|e| KiwiError::DecodeError(format!("Failed to read kind byte: {:?}", e)))?;
        let kind = match kind_byte {
            0 => DefinitionKind::Enum,
            1 => DefinitionKind::Struct,
            2 => DefinitionKind::Message,
            _ => {
                return Err(KiwiError::DecodeError(format!(
                    "Invalid DefinitionKind value: {}",
                    kind_byte
                )))
            }
        };

        let field_count = bb
            .read_var_uint()
            .map_err(|e| KiwiError::DecodeError(format!("Failed to read field count: {:?}", e)))?;

        let mut fields_temp: Vec<FieldTemp> = Vec::with_capacity(field_count as usize);
        for _ in 0..field_count {
            let field_name = bb
                .read_string()
                .map_err(|e| {
                    KiwiError::DecodeError(format!("Failed to read field name: {:?}", e))
                })?
                .into_owned();

            let type_num = bb
                .read_var_int()
                .map_err(|e| KiwiError::DecodeError(format!("Failed to read type_num: {:?}", e)))?;

            let is_array_byte = bb
                .read_byte()
                .map_err(|e| KiwiError::DecodeError(format!("Failed to read is_array byte: {:?}", e)))?;
            let is_array = (is_array_byte & 1) != 0;

            let reserved_index = bb
                .read_var_uint()
                .map_err(|e| KiwiError::DecodeError(format!("Failed to read reserved_index: {:?}", e)))?;

            fields_temp.push(FieldTemp {
                name:           field_name,
                type_num,
                is_array,
                reserved_index,
            });
        }

        definitions_temp.push(DefinitionTemp {
            name:   definition_name,
            kind,
            fields: fields_temp,
        });
    }

    // Build the final definitions with resolved type names
    let native_types: Vec<&str> = NATIVE_TYPES.iter().cloned().collect();
    let mut definitions: Vec<crate::types::Definition> =
        Vec::with_capacity(definition_count as usize);

    for def_temp in &definitions_temp {
        let mut fields = Vec::with_capacity(def_temp.fields.len());

        for field_temp in &def_temp.fields {
            // Resolve the type string (None for enums)
            let type_resolved: Option<String> = if def_temp.kind == DefinitionKind::Enum {
                None
            } else {
                if field_temp.type_num < 0 {
                    // Negative => native type
                    let index = (!field_temp.type_num) as usize;
                    if index >= native_types.len() {
                        return Err(KiwiError::DecodeError(format!(
                            "Invalid native type index {} for field {}",
                            field_temp.type_num, field_temp.name
                        )));
                    }
                    Some(native_types[index].to_string())
                } else {
                    // Non‐negative => an index into definitions_temp
                    let index = field_temp.type_num as usize;
                    if index >= definitions_temp.len() {
                        return Err(KiwiError::DecodeError(format!(
                            "Invalid definition index {} for field {}",
                            field_temp.type_num, field_temp.name
                        )));
                    }
                    Some(definitions_temp[index].name.clone())
                }
            };

            fields.push(Field {
                name:           field_temp.name.clone(),
                line:           0,
                column:         0,
                type_:          type_resolved,
                is_array:       field_temp.is_array,
                is_deprecated:  false, // no deprecation in binary format
                reserved_index: field_temp.reserved_index as i32,
            });
        }

        definitions.push(crate::types::Definition {
            name:    def_temp.name.clone(),
            line:    0,
            column:  0,
            kind:    def_temp.kind.clone(),
            fields,
        });
    }

    // Package is never encoded in the binary format
    Ok(Schema {
        package:    None,
        definitions,
    })
}

/// Encode a `Schema` into bytes. Returns `Err(KiwiError::EncodeError)` if any field's type is invalid.
pub fn encode_binary_schema(schema: &Schema) -> Result<Vec<u8>, KiwiError> {
    use std::collections::HashMap;

    struct Writer {
        buffer: Vec<u8>,
    }

    impl Writer {
        fn new() -> Self {
            Writer { buffer: Vec::new() }
        }

        fn write_var_uint(&mut self, mut value: u32) {
            loop {
                let mut byte = (value & 0x7F) as u8;
                value >>= 7;
                if value != 0 {
                    byte |= 0x80;
                    self.buffer.push(byte);
                } else {
                    self.buffer.push(byte);
                    break;
                }
            }
        }

        fn write_var_int(&mut self, value: i32) {
            let zigzag = if value < 0 {
                !((value as u32) << 1)
            } else {
                (value as u32) << 1
            };
            self.write_var_uint(zigzag);
        }

        fn write_string(&mut self, val: &str) {
            self.buffer.extend_from_slice(val.as_bytes());
            self.buffer.push(0);
        }

        fn write_byte(&mut self, b: u8) {
            self.buffer.push(b);
        }
    }

    let mut writer = Writer::new();
    let definition_count = schema.definitions.len();
    writer.write_var_uint(definition_count as u32);

    // Build a map: name -> index
    let mut definition_index_map = HashMap::new();
    for (i, def) in schema.definitions.iter().enumerate() {
        definition_index_map.insert(def.name.clone(), i);
    }

    let native_types: Vec<&str> = NATIVE_TYPES.iter().cloned().collect();

    for def in &schema.definitions {
        // Write name
        writer.write_string(&def.name);

        // Write kind byte
        let kind_byte = match def.kind {
            DefinitionKind::Enum    => 0,
            DefinitionKind::Struct  => 1,
            DefinitionKind::Message => 2,
        };
        writer.write_byte(kind_byte);

        // Write field count
        let field_count = def.fields.len();
        writer.write_var_uint(field_count as u32);

        for field in &def.fields {
            // Field name
            writer.write_string(&field.name);

            // Determine type_num
            let type_num: i32 = if def.kind == DefinitionKind::Enum {
                0
            } else if let Some(ref type_str) = field.type_ {
                if let Some(native_idx) = native_types.iter().position(|&t| t == type_str.as_str())
                {
                    !(native_idx as i32) // negative for native type
                } else if let Some(&def_idx) = definition_index_map.get(type_str) {
                    def_idx as i32 // positive for user defs
                } else {
                    return Err(KiwiError::EncodeError(format!(
                        "Type '{}' not found in native types or definitions",
                        type_str
                    )));
                }
            } else {
                0
            };

            writer.write_var_int(type_num);

            // is_array
            let is_array_byte = if field.is_array { 1 } else { 0 };
            writer.write_byte(is_array_byte);

            // reserved_index
            writer.write_var_uint(field.reserved_index as u32);
        }
    }

    Ok(writer.buffer)
}
