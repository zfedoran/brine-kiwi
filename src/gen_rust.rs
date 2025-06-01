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
