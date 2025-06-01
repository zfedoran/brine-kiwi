
/FILE: src/types.rs

#[derive(Debug, PartialEq)]
pub struct Schema {
    pub package: Option<String>,
    pub definitions: Vec<Definition>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DefinitionKind {
    Enum = 0,
    Struct = 1,
    Message = 2,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub line: usize,
    pub column: usize,
    pub type_: Option<String>,
    pub is_array: bool,
    pub is_deprecated: bool,
    pub reserved_index: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    pub name: String,
    pub line: usize,
    pub column: usize,
    pub kind: DefinitionKind,
    pub fields: Vec<Field>,
}




/FILE: src/compiler.rs

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




/FILE: src/lib.rs

pub mod types;
pub mod utils;
pub mod tokenizer;
pub mod parser;
pub mod verifier;
pub mod compiler;
pub mod gen_rust;

pub use compiler::compile_schema;




/FILE: src/verifier.rs

use std::collections::HashMap;
use crate::{
    types::{Schema, Definition, DefinitionKind},
    utils::{quote, error},
};

pub const RESERVED_NAMES: [&str; 2] = ["ByteBuffer", "package"];
pub const NATIVE_TYPES: [&str; 8] = [
    "bool",
    "byte",
    "int",
    "uint",
    "float",
    "string",
    "int64",
    "uint64",
];

pub fn verify_schema(schema: &Schema) {
    let mut defined_types: Vec<String> = NATIVE_TYPES.iter().map(|s| s.to_string()).collect();
    let mut definitions_map: HashMap<String, &Definition> = HashMap::new();

    // Define definitions
    for def in &schema.definitions {
        if defined_types.contains(&def.name) {
            error(
                &format!(
                    "The type {} is defined twice",
                    quote(&def.name)
                ),
                def.line,
                def.column,
            );
        }
        if RESERVED_NAMES.contains(&def.name.as_str()) {
            error(
                &format!(
                    "The type name {} is reserved",
                    quote(&def.name)
                ),
                def.line,
                def.column,
            );
        }
        defined_types.push(def.name.clone());
        definitions_map.insert(def.name.clone(), def);
    }

    // Check fields
    for def in &schema.definitions {
        if let DefinitionKind::Enum = def.kind {
            continue;
        }
        if def.fields.is_empty() {
            continue;
        }

        // Check types
        for field in &def.fields {
            if let Some(ref ty) = field.type_ {
                if !defined_types.contains(ty) {
                    error(
                        &format!(
                            "The type {} is not defined for field {}",
                            quote(ty),
                            quote(&field.name)
                        ),
                        field.line,
                        field.column,
                    );
                }
            }
        }

        // Check values
        let mut values = Vec::new();
        for field in &def.fields {
            if values.contains(&field.reserved_index) {
                error(
                    &format!(
                        "The id for field {} is used twice",
                        quote(&field.name)
                    ),
                    field.line,
                    field.column,
                );
            }
            if field.reserved_index <= 0 {
                error(
                    &format!(
                        "The id for field {} must be positive",
                        quote(&field.name)
                    ),
                    field.line,
                    field.column,
                );
            }
            if field.reserved_index > def.fields.len() as i32 {
                error(
                    &format!(
                        "The id for field {} cannot be larger than {}",
                        quote(&field.name),
                        def.fields.len()
                    ),
                    field.line,
                    field.column,
                );
            }
            values.push(field.reserved_index);
        }
    }

    // Check that structs don't contain themselves recursively
    let mut state: HashMap<String, u8> = HashMap::new();
    fn check_recursion(
        name: &str,
        definitions_map: &HashMap<String, &Definition>,
        state: &mut HashMap<String, u8>,
    ) {
        let definition = match definitions_map.get(name) {
            Some(def) => def,
            None => return, // Types not defined or not structs are ignored
        };
        if let DefinitionKind::Struct = definition.kind {
            if let Some(&s) = state.get(name) {
                if s == 1 {
                    error(
                        &format!(
                            "Recursive nesting of {} is not allowed",
                            quote(name)
                        ),
                        definition.line,
                        definition.column,
                    );
                } else if s == 2 {
                    return;
                }
            }

            state.insert(name.to_string(), 1);
            for field in &definition.fields {
                if !field.is_array {
                    if let Some(ref ty) = field.type_ {
                        check_recursion(ty, definitions_map, state);
                    }
                }
            }
            state.insert(name.to_string(), 2);
        }
    }

    for def in &schema.definitions {
        check_recursion(&def.name, &definitions_map, &mut state);
    }
}




/FILE: src/gen_rust.rs

use crate::types::{Definition, DefinitionKind, Schema};
use std::collections::HashMap;
use crate::verifier::NATIVE_TYPES;

/// Converts a string to PascalCase.
/// - If the string contains underscores, it splits on underscores and converts each word
///   so that its first letter is uppercase and the rest lowercase.
/// - If the string does not contain underscores and is fully uppercase, it converts it
///   so that only the first letter is uppercase and the rest are lowercase.
/// - Otherwise, it ensures only the first letter is uppercase.
fn to_pascal_case(s: &str) -> String {
    if s.contains('_') {
        s.split('_')
         .filter(|word| !word.is_empty())
         .map(|word| {
             let mut chars = word.chars();
             match chars.next() {
                 None => String::new(),
                 Some(first) => first.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
             }
         })
         .collect::<String>()
    } else {
        if s == s.to_uppercase() {
            // If the input is fully uppercase (e.g. "SIGNAL"), convert all letters except the first to lowercase.
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        } else {
            // Otherwise, preserve the casing of the rest of the string.
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + chars.as_str(),
            }
        }
    }
}

/// Converts a string to snake_case.
/// This implementation avoids inserting underscores between consecutive uppercase letters,
/// so that acronyms remain intact (e.g. "sessionID" becomes "session_id").
fn to_snake_case(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut snake = String::new();
    for i in 0..chars.len() {
        let c = chars[i];
        if c.is_uppercase() {
            if i > 0 {
                let prev = chars[i - 1];
                // Insert an underscore if the previous character is not uppercase,
                // or if the next character exists and is lowercase.
                if !prev.is_uppercase() || (i + 1 < chars.len() && chars[i + 1].is_lowercase()) {
                    snake.push('_');
                }
            }
            snake.push(c.to_lowercase().next().unwrap());
        } else {
            snake.push(c);
        }
    }
    snake
}

/// Maps schema types to Rust types.
/// - If `is_array` is true, returns `Vec<T>` or `Option<Vec<T>>` based on `is_message`.
/// - If `is_message` is true, wraps the type in `Option<T>`.
fn map_type(type_name: &str, is_message: bool, is_array: bool) -> String {
    let rust_type = match type_name {
        "bool" => "bool".to_string(),
        "byte" => "u8".to_string(),
        "int" => "i32".to_string(),
        "uint" => "u32".to_string(),
        "float" => "f32".to_string(),
        "string" => "String".to_string(),
        "int64" => "i64".to_string(),
        "uint64" => "u64".to_string(),
        other => to_pascal_case(other),
    };

    if is_array {
        if is_message {
            format!("Option<Vec<{}>>", rust_type)
        } else {
            format!("Vec<{}>", rust_type)
        }
    } else {
        if is_message {
            format!("Option<{}>", rust_type)
        } else {
            rust_type
        }
    }
}

/// Returns the full conversion method call for a given type as a string.
/// For example, for "string" it returns "as_string().to_string()".
fn conversion_method(type_name: &str) -> String {
    match type_name {
        "bool" => "as_bool()".to_string(),
        "byte" => "as_byte()".to_string(),
        "int" => "as_int()".to_string(),
        "uint" => "as_uint()".to_string(),
        "float" => "as_float()".to_string(),
        "string" => "as_string().to_string()".to_string(),
        "int64" => "as_int64()".to_string(),
        "uint64" => "as_uint64()".to_string(),
        _ => "as_string()".to_string(), // Default conversion for unsupported types
    }
}

/// Escapes Rust reserved keywords by suffixing with an underscore.
fn escape_rust_keyword(s: &str) -> String {
    let keywords = [
        "as", "break", "const", "continue", "crate", "else",
        "enum", "extern", "false", "fn", "for", "if", "impl",
        "in", "let", "loop", "match", "mod", "move", "mut",
        "pub", "ref", "return", "self", "Self", "static",
        "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while",
    ];
    if keywords.contains(&s) {
        format!("{}_", s)
    } else {
        s.to_string()
    }
}

/// Compiles the entire schema into Rust type definitions as a string,
/// including `FromKiwi` implementations and Serde attributes.
pub fn compile_schema_to_rust(schema: &Schema) -> String {
    let mut definitions_map: HashMap<String, Definition> = HashMap::new();
    let package = schema.package.clone();
    let mut rust_code: Vec<String> = Vec::new();

    // Start module
    if let Some(name) = package.clone() {
        rust_code.push(format!("pub mod {} {{", to_pascal_case(&name)));
    }

    // Add necessary imports
    rust_code.push("use kiwi_schema::Value;".to_string());
    rust_code.push("use brine_kiwi::FromKiwi;".to_string());
    rust_code.push("".to_string());

    // Add Serde imports
    rust_code.push("use serde::Serialize;".to_string());
    rust_code.push("use serde_with::skip_serializing_none;".to_string());
    rust_code.push("".to_string());

    // Collect definitions
    for definition in &schema.definitions {
        definitions_map.insert(definition.name.clone(), definition.clone());
    }

    for definition in &schema.definitions {
        match definition.kind {
            DefinitionKind::Enum => {
                rust_code.push(generate_enum(definition));
            },
            DefinitionKind::Struct => {
                rust_code.push(generate_struct(definition, false));
            },
            DefinitionKind::Message => {
                rust_code.push(generate_struct(definition, true));
            },
        }
    }

    if package.is_some() {
        rust_code.push("}".to_string());
    }

    rust_code.join("\n")
}

/// Generates Rust code for an enum based on the schema definition.
/// Derives Serialize for enums.
fn generate_enum(definition: &Definition) -> String {
    let enum_name = to_pascal_case(&definition.name);
    let mut variants = Vec::new();

    for field in &definition.fields {
        let variant_name = escape_rust_keyword(&to_pascal_case(&field.name));
        if field.is_deprecated {
            variants.push(format!("    #[deprecated]\n    {},", variant_name));
        } else {
            variants.push(format!("    {},", variant_name));
        }
    }

    let derived = "#[derive(Debug, Clone, PartialEq, Serialize)]";
    let enum_def = format!(
        "{}\npub enum {} {{\n{}\n}}\n",
        derived,
        enum_name,
        variants.join("\n")
    );

    let from_kiwi_impl = generate_enum_from_kiwi(definition);

    format!("{}\n{}", enum_def, from_kiwi_impl)
}

/// Generates the `FromKiwi` implementation for an enum.
/// The match arms compare against the original (uppercased) strings,
/// but the returned variant names are in PascalCase.
fn generate_enum_from_kiwi(definition: &Definition) -> String {
    let enum_name = to_pascal_case(&definition.name);
    let mut match_arms = Vec::new();

    for field in &definition.fields {
        let variant_name = escape_rust_keyword(&to_pascal_case(&field.name));
        match_arms.push(format!(
            "        \"{}\" => {}::{},",
            field.name.to_uppercase(),
            enum_name,
            variant_name
        ));
    }
    // Default to the first variant if no match is found.
    let default_variant = escape_rust_keyword(&to_pascal_case(&definition.fields[0].name));
    match_arms.push(format!(
        "        _ => {}::{},",
        enum_name,
        default_variant
    ));

    let impl_block = format!(
        "impl FromKiwi for {} {{\n    fn from_kiwi(value: &Value) -> Self {{\n        let field = value.as_string();\n        match field {{\n{}\n        }}\n    }}\n}}\n",
        enum_name,
        match_arms.join("\n")
    );

    impl_block
}

/// Generates Rust code for a struct or message based on the schema definition.
/// Applies `#[skip_serializing_none]` and derives Serialize.
/// If `is_message` is true, fields are wrapped in `Option<T>`.
fn generate_struct(definition: &Definition, is_message: bool) -> String {
    let struct_name = to_pascal_case(&definition.name);
    let mut fields = Vec::new();

    for field in &definition.fields {
        // The JSON key remains as the original, but the Rust field name is converted to snake_case.
        let rust_field_name = escape_rust_keyword(&to_snake_case(&field.name));
        let field_type = match &field.type_ {
            Some(t) => map_type(t, is_message && definition.kind == DefinitionKind::Message, field.is_array),
            None => {
                if definition.kind == DefinitionKind::Enum {
                    "i32".to_string() // Adjust as necessary.
                } else {
                    "String".to_string() // Default type.
                }
            },
        };
        let mut field_line = String::new();
        if field.is_deprecated {
            field_line.push_str("    #[deprecated]\n");
        }
        field_line.push_str(&format!("    pub {}: {},", rust_field_name, field_type));
        fields.push(field_line);
    }

    // Apply `#[skip_serializing_none]` and derive Serialize along with existing traits
    let derived = "#[derive(Debug, Clone, PartialEq, Default, Serialize)]";
    let serde_attribute = "#[skip_serializing_none]";
    let struct_def = format!(
        "{}\n{}\npub struct {} {{\n{}\n}}\n",
        serde_attribute,
        derived,
        struct_name,
        fields.join("\n")
    );

    let from_kiwi_impl = generate_struct_from_kiwi(definition, is_message);

    format!("{}\n{}", struct_def, from_kiwi_impl)
}

/// Generates the `FromKiwi` implementation for a struct or message.
fn generate_struct_from_kiwi(definition: &Definition, is_message: bool) -> String {
    let struct_name = to_pascal_case(&definition.name);
    let instance_name = to_snake_case(&struct_name);
    let mut field_assignments = Vec::new();

    for field in &definition.fields {
        let original_field_name = &field.name; // For JSON keys.
        let rust_field_name = escape_rust_keyword(&to_snake_case(&field.name)); // For Rust identifiers in snake_case.
        let type_name = field.type_.as_deref().unwrap_or("");
        let is_array = field.is_array;
        let is_base_type = NATIVE_TYPES.contains(&type_name);

        if is_array {
            if is_base_type {
                if is_message && definition.kind == DefinitionKind::Message {
                    field_assignments.push(format!(
                        "    if let Some(value) = value.get(\"{}\") {{\n        let mut vec = Vec::new();\n        for item in value.as_array() {{\n            vec.push(item.{});\n        }}\n        {}.{} = Some(vec);\n    }}",
                        original_field_name,
                        conversion_method(type_name),
                        instance_name,
                        rust_field_name
                    ));
                } else {
                    field_assignments.push(format!(
                        "    if let Some(value) = value.get(\"{}\") {{\n        let mut vec = Vec::new();\n        for item in value.as_array() {{\n            vec.push(item.{});\n        }}\n        {}.{} = vec;\n    }} else {{\n        panic!(\"Missing required field {}\");\n    }}",
                        original_field_name,
                        conversion_method(type_name),
                        instance_name,
                        rust_field_name,
                        original_field_name
                    ));
                }
            } else {
                if is_message && definition.kind == DefinitionKind::Message {
                    field_assignments.push(format!(
                        "    if let Some(value) = value.get(\"{}\") {{\n        let mut vec = Vec::new();\n        for item in value.as_array() {{\n            vec.push({}::from_kiwi(item));\n        }}\n        {}.{} = Some(vec);\n    }}",
                        original_field_name,
                        to_pascal_case(type_name),
                        instance_name,
                        rust_field_name
                    ));
                } else {
                    field_assignments.push(format!(
                        "    if let Some(value) = value.get(\"{}\") {{\n        let mut vec = Vec::new();\n        for item in value.as_array() {{\n            vec.push({}::from_kiwi(item));\n        }}\n        {}.{} = vec;\n    }} else {{\n        panic!(\"Missing required field {}\");\n    }}",
                        original_field_name,
                        to_pascal_case(type_name),
                        instance_name,
                        rust_field_name,
                        original_field_name
                    ));
                }
            }
        } else {
            if is_message && definition.kind == DefinitionKind::Message {
                if is_base_type {
                    field_assignments.push(format!(
                        "    if let Some(value) = value.get(\"{}\") {{\n        {}.{} = Some(value.{});\n    }}",
                        original_field_name,
                        instance_name,
                        rust_field_name,
                        conversion_method(type_name)
                    ));
                } else {
                    field_assignments.push(format!(
                        "    if let Some(value) = value.get(\"{}\") {{\n        {}.{} = Some({}::from_kiwi(value));\n    }}",
                        original_field_name,
                        instance_name,
                        rust_field_name,
                        to_pascal_case(type_name)
                    ));
                }
            } else {
                if is_base_type {
                    field_assignments.push(format!(
                        "    if let Some(value) = value.get(\"{}\") {{\n        {}.{} = value.{};\n    }} else {{\n        panic!(\"Missing required field {}\");\n    }}",
                        original_field_name,
                        instance_name,
                        rust_field_name,
                        conversion_method(type_name),
                        original_field_name
                    ));
                } else {
                    field_assignments.push(format!(
                        "    if let Some(value) = value.get(\"{}\") {{\n        {}.{} = {}::from_kiwi(value);\n    }} else {{\n        panic!(\"Missing required field {}\");\n    }}",
                        original_field_name,
                        instance_name,
                        rust_field_name,
                        to_pascal_case(type_name),
                        original_field_name
                    ));
                }
            }
        }
    }

    let mut impl_lines = Vec::new();

    impl_lines.push(format!("impl FromKiwi for {} {{", struct_name));
    impl_lines.push("    fn from_kiwi(value: &Value) -> Self {".to_string());
    impl_lines.push(format!("        let mut {} = Self::default();", instance_name));
    impl_lines.push("".to_string());

    for assignment in field_assignments {
        impl_lines.push(assignment);
        impl_lines.push("".to_string());
    }

    impl_lines.push(format!("        {}", instance_name));
    impl_lines.push("    }".to_string());
    impl_lines.push("}".to_string());

    impl_lines.join("\n")
}




/FILE: src/traits.rs

use kiwi_schema::Value;

pub trait FromKiwi {
    fn from_kiwi(value: &Value) -> Self;
}




/FILE: src/tokenizer.rs

use regex::Regex;
use lazy_static::lazy_static;
use crate::utils::{quote, error};

lazy_static! {
    pub static ref token_regex: Regex = Regex::new(r"((?:-|\b)\d+\b|[=;{}]|\[\]|\[deprecated\]|\b[A-Za-z_][A-Za-z0-9_]*\b|//.*|\s+)").unwrap();
    pub static ref whitespace_regex: Regex = Regex::new(r"^(//.*|\s+)$").unwrap();
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub text: String,
    pub line: usize,
    pub column: usize,
}

pub fn tokenize_schema(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut line = 1;
    let mut column = 1;
    let mut last_end = 0;

    for mat in token_regex.find_iter(text) {
        let start = mat.start();
        let end = mat.end();
        let part = mat.as_str();

        if start > last_end {
            // There is some unexpected text
            let unexpected = &text[last_end..start];
            error(&format!("Syntax error {}", quote(unexpected)), line, column);
        }

        if !whitespace_regex.is_match(part) && !part.starts_with("//") {
            tokens.push(Token {
                text: part.to_string(),
                line,
                column,
            });
        }

        // Update line and column
        let newline_count = part.matches('\n').count();
        if newline_count > 0 {
            line += newline_count;
            if let Some(last_line_part) = part.split('\n').last() {
                column = last_line_part.len() + 1;
            }
        } else {
            column += part.len();
        }

        last_end = end;
    }

    if last_end != text.len() {
        let unexpected = &text[last_end..];
        error(&format!("Syntax error {}", quote(unexpected)), line, column);
    }

    // Add end-of-file token
    tokens.push(Token {
        text: "".to_string(),
        line,
        column,
    });

    tokens
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let input = "int x = 10;";
        let expected_tokens = vec![
            Token { text: "int".to_string(), line:1, column:1 },
            Token { text: "x".to_string(), line:1, column:5 },
            Token { text: "=".to_string(), line:1, column:7 },
            Token { text: "10".to_string(), line:1, column:9 },
            Token { text: ";".to_string(), line:1, column:11 },
            Token { text: "".to_string(), line:1, column:12 }, // EOF token
        ];

        let tokens = tokenize_schema(input);
        assert_eq!(tokens, expected_tokens);
    }

    #[test]
    fn test_tokenize_with_deprecated_tag() {
        let input = "[deprecated]";
        let expected_tokens = vec![
            Token { text: "[deprecated]".to_string(), line:1, column:1 },
            Token { text: "".to_string(), line:1, column:13 }, // EOF token
        ];

        let tokens = tokenize_schema(input);
        assert_eq!(tokens, expected_tokens);
    }

    #[test]
    fn test_tokenize_reserved_names() {
        let input = "ByteBuffer package";
        let expected_tokens = vec![
            Token { text: "ByteBuffer".to_string(), line:1, column:1 },
            Token { text: "package".to_string(), line:1, column:12 },
            Token { text: "".to_string(), line:1, column:19 }, // EOF token
        ];

        let tokens = tokenize_schema(input);
        assert_eq!(tokens, expected_tokens);
    }

    #[test]
    #[should_panic(expected = "Syntax error")]
    fn test_tokenize_unexpected_text() {
        let input = "int x = 10 @";
        tokenize_schema(input); // This should panic due to the unexpected "@"
    }
}





/FILE: src/parser.rs

use crate::{
    tokenizer::Token,
    types::{Definition, DefinitionKind, Field, Schema},
    utils::{error, quote},
};

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref identifier: Regex = Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").unwrap();
    static ref equals: Regex = Regex::new(r"^=$").unwrap();
    static ref semicolon: Regex = Regex::new(r"^;$").unwrap();
    static ref integer: Regex = Regex::new(r"^-?\d+$").unwrap();
    static ref left_brace: Regex = Regex::new(r"^\{$").unwrap();
    static ref right_brace: Regex = Regex::new(r"^\}$").unwrap();
    static ref array_token: Regex = Regex::new(r"^\[\]$").unwrap();
    static ref enum_keyword: Regex = Regex::new(r"^enum$").unwrap();
    static ref struct_keyword: Regex = Regex::new(r"^struct$").unwrap();
    static ref message_keyword: Regex = Regex::new(r"^message$").unwrap();
    static ref package_keyword: Regex = Regex::new(r"^package$").unwrap();
    static ref deprecated_token: Regex = Regex::new(r"^\[deprecated\]$").unwrap();
    static ref end_of_file: Regex = Regex::new(r"^$").unwrap();
}

pub fn parse_schema(tokens: &[Token]) -> Schema {
    let mut definitions = Vec::new();
    let mut package_text = None;
    let mut index = 0;

    fn current_token(tokens: &[Token], index: usize) -> &Token {
        tokens.get(index).expect("Unexpected end of tokens")
    }

    fn eat(tokens: &[Token], index: &mut usize, test: &Regex) -> bool {
        if test.is_match(&current_token(tokens, *index).text) {
            *index += 1;
            true
        } else {
            false
        }
    }

    fn expect(tokens: &[Token], index: &mut usize, test: &Regex, expected: &str) {
        if !eat(tokens, index, test) {
            let token = current_token(tokens, *index);
            error(
                &format!("Expected {} but found {}", expected, quote(&token.text)),
                token.line,
                token.column,
            );
        }
    }

    fn unexpected_token(tokens: &[Token], index: &mut usize) -> ! {
        let token = current_token(tokens, *index);
        error(
            &format!("Unexpected token {}", quote(&token.text)),
            token.line,
            token.column,
        );
    }

    // Handle package declaration
    if eat(tokens, &mut index, &package_keyword) {
        if index >= tokens.len() {
            error("Expected identifier after package", 0, 0);
        }
        let pkg_token = current_token(tokens, index);
        expect(tokens, &mut index, &identifier, "identifier");
        package_text = Some(pkg_token.text.clone());
        expect(tokens, &mut index, &semicolon, "\";\"");
    }

    // Parse definitions
    while index < tokens.len() && !eat(tokens, &mut index, &end_of_file) {
        let kind = if eat(tokens, &mut index, &enum_keyword) {
            DefinitionKind::Enum
        } else if eat(tokens, &mut index, &struct_keyword) {
            DefinitionKind::Struct
        } else if eat(tokens, &mut index, &message_keyword) {
            DefinitionKind::Message
        } else {
            unexpected_token(tokens, &mut index);
        };

        // All definitions start with a name
        let name_token = current_token(tokens, index);
        expect(tokens, &mut index, &identifier, "identifier");
        expect(tokens, &mut index, &left_brace, "\"{\"");

        // Parse fields
        let mut fields = Vec::new();
        while !eat(tokens, &mut index, &right_brace) {
            let mut type_opt = None;
            let mut is_array = false;
            let mut is_deprecated = false;

            if kind != DefinitionKind::Enum {
                // Get type
                let type_token = current_token(tokens, index);
                expect(tokens, &mut index, &identifier, "identifier");
                if eat(tokens, &mut index, &array_token) {
                    is_array = true;
                }
                type_opt = Some(type_token.text.clone());
            }

            // Field name
            let field_token = current_token(tokens, index);
            expect(tokens, &mut index, &identifier, "identifier");

            // Structs don't have explicit values
            let value = if kind != DefinitionKind::Struct {
                expect(tokens, &mut index, &equals, "\"=\"");
                let value_token = current_token(tokens, index);
                expect(tokens, &mut index, &integer, "integer");
                let parsed = value_token.text.parse::<i32>().unwrap_or_else(|_| {
                    error(
                        &format!("Invalid integer {}", quote(&value_token.text)),
                        value_token.line,
                        value_token.column,
                    )
                });
                parsed
            } else {
                // For struct, value is fields.len() + 1
                fields.len() as i32 + 1
            };

            // Check for deprecated
            if eat(tokens, &mut index, &deprecated_token) {
                if kind != DefinitionKind::Message {
                    let deprecated = current_token(tokens, index - 1);
                    error(
                        "Cannot deprecate this field",
                        deprecated.line,
                        deprecated.column,
                    );
                }
                is_deprecated = true;
            }

            expect(tokens, &mut index, &semicolon, "\";\"");

            // Set value if not struct
            let final_value = if kind != DefinitionKind::Struct {
                value
            } else {
                fields.len() as i32 + 1
            };

            fields.push(Field {
                name: field_token.text.clone(),
                line: field_token.line,
                column: field_token.column,
                type_: type_opt,
                is_array,
                is_deprecated,
                reserved_index: final_value,
            });
        }

        definitions.push(Definition {
            name: name_token.text.clone(),
            line: name_token.line,
            column: name_token.column,
            kind,
            fields,
        });
    }

    Schema {
        package: package_text,
        definitions,
    }
}





/FILE: src/utils.rs

use serde_json;

pub fn quote(text: &str) -> String {
    serde_json::to_string(text).unwrap()
}

pub fn error(msg: &str, line: usize, column: usize) -> ! {
    panic!("Error at line {}, column {}: {}", line, column, msg);
}



