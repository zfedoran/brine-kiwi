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
