use crate::types::{Definition, DefinitionKind, Schema};
use crate::verifier::NATIVE_TYPES;
use std::collections::HashMap;

/// Converts a string to PascalCase.
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
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        } else {
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + chars.as_str(),
            }
        }
    }
}

/// Converts a string to snake_case.
fn to_snake_case(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut snake = String::new();
    for i in 0..chars.len() {
        let c = chars[i];
        if c.is_uppercase() {
            if i > 0 {
                let prev = chars[i - 1];
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
fn map_type(type_name: &str, is_message: bool, is_array: bool) -> String {
    let rust_type = match type_name {
        "bool"   => "bool".to_string(),
        "byte"   => "u8".to_string(),
        "int"    => "i32".to_string(),
        "uint"   => "u32".to_string(),
        "float"  => "f32".to_string(),
        "string" => "String".to_string(),
        "int64"  => "i64".to_string(),
        "uint64" => "u64".to_string(),
        other    => to_pascal_case(other),
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

/// Returns the correct `as_...()` call on a `Value`.
fn conversion_method(type_name: &str) -> String {
    match type_name {
        "bool"   => "as_bool()".to_string(),
        "byte"   => "as_byte()".to_string(),
        "int"    => "as_int()".to_string(),
        "uint"   => "as_uint()".to_string(),
        "float"  => "as_float()".to_string(),
        "string" => "as_string().to_string()".to_string(),
        "int64"  => "as_int64()".to_string(),
        "uint64" => "as_uint64()".to_string(),
        _        => "as_string()".to_string(),
    }
}

/// Escape Rust keywords by appending an underscore.
fn escape_rust_keyword(s: &str) -> String {
    let keywords = [
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while",
    ];
    if keywords.contains(&s) {
        format!("{}_", s)
    } else {
        s.to_string()
    }
}

/// Entry point: given a `Schema`, return a `String` containing the entire Rust module.
/// 
/// Each generated `from_kiwi(…)` now returns `Result<_, KiwiError>`.
pub fn compile_schema_to_rust(schema: &Schema) -> String {
    let mut definitions_map: HashMap<String, Definition> = HashMap::new();
    let package = schema.package.clone();
    let mut rust_code: Vec<String> = Vec::new();

    // If there's a package, wrap everything in a `pub mod PascalCaseName { … }`.
    if let Some(ref name) = package {
        rust_code.push(format!("pub mod {} {{", to_pascal_case(name)));
    }

    // Always import `Value` and `FromKiwi`.
    rust_code.push("use kiwi_schema::Value;".to_string());
    rust_code.push("use brine_kiwi::error::KiwiError;".to_string());
    rust_code.push("use brine_kiwi::traits::FromKiwi;".to_string());
    rust_code.push("".to_string());

    // Serde imports
    rust_code.push("use serde::Serialize;".to_string());
    rust_code.push("use serde_with::skip_serializing_none;".to_string());
    rust_code.push("".to_string());

    // Build a lookup map from name → Definition
    for def in &schema.definitions {
        definitions_map.insert(def.name.clone(), def.clone());
    }

    // Now generate code for each definition
    for definition in &schema.definitions {
        match definition.kind {
            DefinitionKind::Enum => {
                rust_code.push(generate_enum(definition));
            }
            DefinitionKind::Struct => {
                rust_code.push(generate_struct(definition, false));
            }
            DefinitionKind::Message => {
                rust_code.push(generate_struct(definition, true));
            }
        }
    }

    // Close package block if needed
    if package.is_some() {
        rust_code.push("}".to_string());
    }

    rust_code.join("\n")
}

/// Generates a Rust enum + `FromKiwi` impl that returns `Result<…, KiwiError>`.
fn generate_enum(definition: &Definition) -> String {
    let enum_name = to_pascal_case(&definition.name);
    let mut variants = Vec::new();
    for field in &definition.fields {
        let var_name = escape_rust_keyword(&to_pascal_case(&field.name));
        if field.is_deprecated {
            variants.push(format!("    #[deprecated]\n    {},", var_name));
        } else {
            variants.push(format!("    {},", var_name));
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

/// Generates the `FromKiwi` impl for an enum, returning `Result<_, KiwiError>`.
fn generate_enum_from_kiwi(definition: &Definition) -> String {
    let enum_name = to_pascal_case(&definition.name);
    let mut match_arms = Vec::new();

    for field in &definition.fields {
        let variant_name = escape_rust_keyword(&to_pascal_case(&field.name));
        match_arms.push(format!(
            "            \"{}\" => Ok({}::{}),",
            field.name.to_uppercase(),
            enum_name,
            variant_name
        ));
    }

    // If no match, return Err(KiwiError::InvalidEnumVariant(_))
    match_arms.push(format!(
        "            other => Err(KiwiError::InvalidEnumVariant(other.to_string())),"
    ));

    let impl_block = format!(
        r#"impl FromKiwi for {} {{
    fn from_kiwi(value: &Value) -> Result<Self, KiwiError> {{
        let s = value.as_string();
        match s.as_str() {{
{}
        }}
    }}
}}
"#,
        enum_name,
        match_arms.join("\n")
    );

    impl_block
}

/// Generates a Rust struct/message + `FromKiwi` impl that returns `Result<_, KiwiError>`.
fn generate_struct(definition: &Definition, is_message: bool) -> String {
    let struct_name = to_pascal_case(&definition.name);
    let mut fields_code = Vec::new();

    for field in &definition.fields {
        let rust_name = escape_rust_keyword(&to_snake_case(&field.name));
        let field_type = if let Some(ref t) = field.type_ {
            map_type(t, is_message && definition.kind == DefinitionKind::Message, field.is_array)
        } else {
            // If no type, treat as i32 for enums or String for fallback
            if definition.kind == DefinitionKind::Enum {
                "i32".to_string()
            } else {
                "String".to_string()
            }
        };

        let mut line = String::new();
        if field.is_deprecated {
            line.push_str("    #[deprecated]\n");
        }
        line.push_str(&format!("    pub {}: {},", rust_name, field_type));
        fields_code.push(line);
    }

    let derived = "#[derive(Debug, Clone, PartialEq, Default, Serialize)]";
    let serde_attr = "#[skip_serializing_none]";
    let struct_def = format!(
        "{}\n{}\npub struct {} {{\n{}\n}}\n",
        serde_attr,
        derived,
        struct_name,
        fields_code.join("\n")
    );

    let from_kiwi_impl = generate_struct_from_kiwi(definition, is_message);
    format!("{}\n{}", struct_def, from_kiwi_impl)
}

/// Generates the `FromKiwi` impl for a struct/message, returning `Result<..., KiwiError>`.
fn generate_struct_from_kiwi(definition: &Definition, is_message: bool) -> String {
    let struct_name = to_pascal_case(&definition.name);
    let instance = to_snake_case(&struct_name);

    let mut lines = Vec::new();
    lines.push(format!("impl FromKiwi for {} {{", struct_name));
    lines.push("    fn from_kiwi(value: &Value) -> Result<Self, KiwiError> {".into());
    lines.push(format!("        let mut {} = Self::default();", instance));
    lines.push("".into());

    for field in &definition.fields {
        let original = &field.name;
        let rust_name = escape_rust_keyword(&to_snake_case(original));
        let type_name = field.type_.as_deref().unwrap_or("");
        let is_array = field.is_array;
        let is_base = NATIVE_TYPES.contains(&type_name);

        if is_array {
            // Handle array of primitives vs array of messages
            if is_base {
                if is_message && definition.kind == DefinitionKind::Message {
                    // Option<Vec<primitive>>
                    lines.push(format!(
                        "        if let Some(arr) = value.get(\"{}\") {{",
                        original
                    ));
                    lines.push(format!(
                        "            let mut tmp = Vec::new();"
                    ));
                    lines.push(format!(
                        "            for item in arr.as_array() {{ tmp.push(item.{}); }}",
                        conversion_method(type_name)
                    ));
                    lines.push(format!(
                        "            {}.{} = Some(tmp);",
                        instance, rust_name
                    ));
                    lines.push("        }".into());
                } else {
                    // Required Vec<primitive>
                    lines.push(format!(
                        "        if let Some(arr) = value.get(\"{}\") {{",
                        original
                    ));
                    lines.push(format!(
                        "            let mut tmp = Vec::new();"
                    ));
                    lines.push(format!(
                        "            for item in arr.as_array() {{ tmp.push(item.{}); }}",
                        conversion_method(type_name)
                    ));
                    lines.push(format!(
                        "            {}.{} = tmp;",
                        instance, rust_name
                    ));
                    lines.push("        } else {".into());
                    lines.push(format!(
                        "            return Err(KiwiError::MissingField(\"{}\".into()));",
                        original
                    ));
                    lines.push("        }".into());
                }
            } else {
                // array of nested messages
                if is_message && definition.kind == DefinitionKind::Message {
                    // Option<Vec<MessageType>>
                    lines.push(format!(
                        "        if let Some(arr) = value.get(\"{}\") {{",
                        original
                    ));
                    lines.push(format!("            let mut tmp = Vec::new();"));
                    lines.push(format!(
                        "            for item in arr.as_array() {{ tmp.push({}::from_kiwi(item)?); }}",
                        to_pascal_case(type_name)
                    ));
                    lines.push(format!(
                        "            {}.{} = Some(tmp);",
                        instance, rust_name
                    ));
                    lines.push("        }".into());
                } else {
                    // Required Vec<MessageType>
                    lines.push(format!(
                        "        if let Some(arr) = value.get(\"{}\") {{",
                        original
                    ));
                    lines.push(format!("            let mut tmp = Vec::new();"));
                    lines.push(format!(
                        "            for item in arr.as_array() {{ tmp.push({}::from_kiwi(item)?); }}",
                        to_pascal_case(type_name)
                    ));
                    lines.push(format!(
                        "            {}.{} = tmp;",
                        instance, rust_name
                    ));
                    lines.push("        } else {".into());
                    lines.push(format!(
                        "            return Err(KiwiError::MissingField(\"{}\".into()));",
                        original
                    ));
                    lines.push("        }".into());
                }
            }
        } else {
            // Single value (primitive vs nested, required vs optional)
            if is_message && definition.kind == DefinitionKind::Message {
                // Option<...>
                if is_base {
                    lines.push(format!(
                        "        if let Some(val) = value.get(\"{}\") {{",
                        original
                    ));
                    lines.push(format!(
                        "            {}.{} = Some(val.{});",
                        instance, rust_name, conversion_method(type_name)
                    ));
                    lines.push("        }".into());
                } else {
                    lines.push(format!(
                        "        if let Some(val) = value.get(\"{}\") {{",
                        original
                    ));
                    lines.push(format!(
                        "            {}.{} = Some({}::from_kiwi(val)?);",
                        instance, rust_name, to_pascal_case(type_name)
                    ));
                    lines.push("        }".into());
                }
            } else {
                // Required field
                if is_base {
                    lines.push(format!(
                        "        if let Some(val) = value.get(\"{}\") {{",
                        original
                    ));
                    lines.push(format!(
                        "            {}.{} = val.{};",
                        instance, rust_name, conversion_method(type_name)
                    ));
                    lines.push("        } else {".into());
                    lines.push(format!(
                        "            return Err(KiwiError::MissingField(\"{}\".into()));",
                        original
                    ));
                    lines.push("        }".into());
                } else {
                    lines.push(format!(
                        "        if let Some(val) = value.get(\"{}\") {{",
                        original
                    ));
                    lines.push(format!(
                        "            {}.{} = {}::from_kiwi(val)?;",
                        instance, rust_name, to_pascal_case(type_name)
                    ));
                    lines.push("        } else {".into());
                    lines.push(format!(
                        "            return Err(KiwiError::MissingField(\"{}\".into()));",
                        original
                    ));
                    lines.push("        }".into());
                }
            }
        }

        lines.push("".into());
    }

    lines.push(format!("        Ok({})", instance));
    lines.push("    }".into());
    lines.push("}".into());
    lines.join("\n")
}
