use kiwi_schema::ByteBuffer;
use crate::{
    types::{Definition, DefinitionKind, Field, Schema},
    verifier::{ verify_schema, NATIVE_TYPES},
    tokenizer::tokenize_schema,
    parser::parse_schema,
};

pub fn compile_schema(text: &str) -> (Schema, Vec<u8>) {
    let tokens = tokenize_schema(text);
    let schema = parse_schema(&tokens);
    verify_schema(&schema);
    let bin = encode_binary_schema(&schema);
    (schema, bin)
}

pub fn decode_binary_schema(buffer: &[u8]) -> Schema {
    struct FieldTemp {
        name: String,
        type_num: i32,
        is_array: bool,
        reserved_index: u32,
    }

    struct DefinitionTemp {
        name: String,
        kind: DefinitionKind,
        fields: Vec<FieldTemp>,
    }

    let mut bb = ByteBuffer::new(buffer);
    let definition_count = bb.read_var_uint().expect("Failed to read definition count");

    // Define native types
    let native_types: Vec<&str> = NATIVE_TYPES.iter().cloned().collect();

    // Store definitions temporarily with type_num
    let mut definitions_temp: Vec<DefinitionTemp> = Vec::with_capacity(definition_count as usize);

    for _ in 0..definition_count {
        let definition_name = bb
            .read_string()
            .expect("Failed to read definition name")
            .into_owned();

        let kind_byte = bb
            .read_byte()
            .expect("Failed to read kind byte for definition");
        let kind = match kind_byte {
            0 => DefinitionKind::Enum,
            1 => DefinitionKind::Struct,
            2 => DefinitionKind::Message,
            _ => panic!("Invalid DefinitionKind value: {}", kind_byte),
        };

        let field_count = bb.read_var_uint().expect("Failed to read field count");

        let mut fields_temp: Vec<FieldTemp> = Vec::with_capacity(field_count as usize);
        for _ in 0..field_count {
            let field_name = bb
                .read_string()
                .expect("Failed to read field name")
                .into_owned();

            let type_num = bb
                .read_var_int()
                .expect("Failed to read type_num for field");

            let is_array_byte = bb
                .read_byte()
                .expect("Failed to read is_array byte for field");
            let is_array = (is_array_byte & 1) != 0;

            let reserved_index = bb.read_var_uint().expect("Failed to read value for field");

            fields_temp.push(FieldTemp {
                name: field_name,
                type_num,
                is_array,
                reserved_index,
            });
        }

        definitions_temp.push(DefinitionTemp {
            name: definition_name,
            kind,
            fields: fields_temp,
        });
    }

    // Now, build final definitions with resolved type names
    let mut definitions = Vec::with_capacity(definition_count as usize);

    for def_temp in &definitions_temp {
        let mut fields = Vec::with_capacity(def_temp.fields.len());

        for field_temp in &def_temp.fields {
            let type_resolved: Option<String> = if def_temp.kind == DefinitionKind::Enum {
                None
            } else {
                if field_temp.type_num < 0 {
                    let index = (!field_temp.type_num) as usize;
                    if index >= native_types.len() {
                        panic!(
                            "Invalid native type index {} for field {}",
                            field_temp.type_num, field_temp.name
                        );
                    }
                    Some(native_types[index].to_string())
                } else {
                    let index = field_temp.type_num as usize;
                    if index >= definitions_temp.len() {
                        panic!(
                            "Invalid definition index {} for field {}",
                            field_temp.type_num, field_temp.name
                        );
                    }
                    Some(definitions_temp[index].name.clone())
                }
            };

            fields.push(Field {
                name: field_temp.name.clone(),
                line: 0,
                column: 0,
                type_: type_resolved,
                is_array: field_temp.is_array,
                is_deprecated: false, // Assuming no deprecation information in binary schema
                reserved_index: field_temp.reserved_index as i32,
            });
        }

        definitions.push(Definition {
            name: def_temp.name.clone(),
            line: 0,
            column: 0,
            kind: def_temp.kind.clone(),
            fields,
        });
    }

    // The binary schema does not include package information
    Schema {
        package: None,
        definitions,
    }
}

pub fn encode_binary_schema(schema: &Schema) -> Vec<u8> {
    use std::collections::HashMap;

    // Helper Writer struct for encoding
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
            // ZigZag encoding
            let zigzag = if value < 0 {
                !((value as u32) << 1)
            } else {
                (value as u32) << 1
            };
            self.write_var_uint(zigzag);
        }

        fn write_string(&mut self, value: &str) {
            self.buffer.extend_from_slice(value.as_bytes());
            self.buffer.push(0); // Null-terminated
        }

        fn write_byte(&mut self, value: u8) {
            self.buffer.push(value);
        }
    }

    let mut writer = Writer::new();

    // Write definition count
    let definition_count = schema.definitions.len();
    writer.write_var_uint(definition_count as u32);

    // Build a map from definition name to index for quick lookup
    let mut definition_index_map: HashMap<String, usize> = HashMap::new();
    for (i, def) in schema.definitions.iter().enumerate() {
        definition_index_map.insert(def.name.clone(), i);
    }

    // Define native types for reference
    let native_types: Vec<&str> = NATIVE_TYPES.iter().cloned().collect();

    for def in &schema.definitions {
        // Write definition name
        writer.write_string(&def.name);

        // Write kind as byte
        let kind_byte = match def.kind {
            DefinitionKind::Enum => 0,
            DefinitionKind::Struct => 1,
            DefinitionKind::Message => 2,
        };
        writer.write_byte(kind_byte);

        // Write field count
        let field_count = def.fields.len();
        writer.write_var_uint(field_count as u32);

        for field in &def.fields {
            // Write field name
            writer.write_string(&field.name);

            // Determine and write type_num
            let type_num: i32 = if def.kind == DefinitionKind::Enum {
                0 // Type_num is ignored for ENUM fields
            } else if let Some(ref type_str) = field.type_ {
                if let Some(native_idx) = native_types.iter().position(|&t| t == type_str.as_str())
                {
                    !(native_idx as i32) // Negative index for native types
                } else if let Some(&def_idx) = definition_index_map.get(type_str) {
                    def_idx as i32 // Positive index for custom definitions
                } else {
                    panic!(
                        "Type '{}' not found in native types or definitions",
                        type_str
                    );
                }
            } else {
                0 // Default value if type is None (should only be for ENUM fields)
            };

            writer.write_var_int(type_num);

            // Write isArray as byte (1 for true, 0 for false)
            let is_array_byte = if field.is_array { 1 } else { 0 };
            writer.write_byte(is_array_byte);

            // Write value as var_uint
            writer.write_var_uint(field.reserved_index as u32);
        }
    }

    writer.buffer
}
