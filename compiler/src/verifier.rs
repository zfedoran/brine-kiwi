use std::collections::HashMap;
use crate::{
    types::{Schema, Definition, DefinitionKind},
    utils::quote,
    error::KiwiError,
};

pub const RESERVED_NAMES: [&str; 2] = ["ByteBuffer", "package"];
pub const NATIVE_TYPES: [&str; 8] = [
    "bool", "byte", "int", "uint", "float", "string", "int64", "uint64",
];

/// Returns `Ok(())` if verification passed, or `Err(KiwiError::VerifierError(_))` otherwise.
pub fn verify_schema(schema: &Schema) -> Result<(), KiwiError> {
    let mut defined_types: Vec<String> = NATIVE_TYPES.iter().map(|s| s.to_string()).collect();
    let mut definitions_map: HashMap<String, &Definition> = HashMap::new();

    // 1) Check duplicate / reserved type names
    for def in &schema.definitions {
        if defined_types.contains(&def.name) {
            return Err(KiwiError::VerifierError(format!(
                "The type {} is defined twice",
                quote(&def.name)
            )));
        }
        if RESERVED_NAMES.contains(&def.name.as_str()) {
            return Err(KiwiError::VerifierError(format!(
                "The type name {} is reserved",
                quote(&def.name)
            )));
        }
        defined_types.push(def.name.clone());
        definitions_map.insert(def.name.clone(), def);
    }

    // 2) Check fields inside each non‐enum definition
    for def in &schema.definitions {
        if let DefinitionKind::Enum = def.kind {
            continue;
        }
        if def.fields.is_empty() {
            continue;
        }

        // Check that each field's type is defined
        for field in &def.fields {
            if let Some(ref ty) = field.type_ {
                if !defined_types.contains(ty) {
                    return Err(KiwiError::VerifierError(format!(
                        "The type {} is not defined for field {}",
                        quote(ty),
                        quote(&field.name)
                    )));
                }
            }
        }

        // Check reserved_index uniqueness and bounds
        let mut values = Vec::new();
        for field in &def.fields {
            if values.contains(&field.reserved_index) {
                return Err(KiwiError::VerifierError(format!(
                    "The id for field {} is used twice",
                    quote(&field.name)
                )));
            }
            if field.reserved_index <= 0 {
                return Err(KiwiError::VerifierError(format!(
                    "The id for field {} must be positive",
                    quote(&field.name)
                )));
            }
            if field.reserved_index > def.fields.len() as i32 {
                return Err(KiwiError::VerifierError(format!(
                    "The id for field {} cannot be larger than {}",
                    quote(&field.name),
                    def.fields.len()
                )));
            }
            values.push(field.reserved_index);
        }
    }

    // 3) Check that structs do not contain themselves recursively
    let mut state: HashMap<String, u8> = HashMap::new();
    fn check_recursion(
        name: &str,
        definitions_map: &HashMap<String, &Definition>,
        state: &mut HashMap<String, u8>,
    ) -> Result<(), KiwiError> {
        let definition = match definitions_map.get(name) {
            Some(def) => def,
            None => return Ok(()),
        };
        if let DefinitionKind::Struct = definition.kind {
            if let Some(&s) = state.get(name) {
                if s == 1 {
                    return Err(KiwiError::VerifierError(format!(
                        "Recursive nesting of {} is not allowed",
                        quote(name)
                    )));
                } else if s == 2 {
                    return Ok(());
                }
            }
            state.insert(name.to_string(), 1);
            for field in &definition.fields {
                if !field.is_array {
                    if let Some(ref ty) = field.type_ {
                        check_recursion(ty, definitions_map, state)?;
                    }
                }
            }
            state.insert(name.to_string(), 2);
        }
        Ok(())
    }

    for def in &schema.definitions {
        check_recursion(&def.name, &definitions_map, &mut state)?;
    }

    Ok(())
}
