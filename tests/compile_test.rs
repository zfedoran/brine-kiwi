#![cfg(test)]

use brine_kiwi::{
    gen_rust::compile_schema_to_rust,
    parser::parse_schema,
    tokenizer::tokenize_schema,
    types::DefinitionKind,
};

#[test]
fn test_parse_schema() {
    let input = r#"
    enum Type {
      FLAT = 0;
      ROUND = 1;
      POINTED = 2;
    }

    struct Color {
      byte red;
      byte green;
      byte blue;
      byte alpha;
    }

    message Example {
      uint clientID = 1;
      Type type = 2;
      Color[] colors = 3;
    }
    "#;

    let tokens = tokenize_schema(input).expect("tokenize_schema failed");
    let schema = parse_schema(&tokens).expect("parse_schema failed");

    // Check package is None
    assert!(schema.package.is_none());

    // Check number of definitions
    assert_eq!(schema.definitions.len(), 3);

    // Check enum Type
    let type_def = &schema.definitions[0];
    assert_eq!(type_def.kind, DefinitionKind::Enum);
    assert_eq!(type_def.name, "Type");
    assert_eq!(type_def.fields.len(), 3);
    assert_eq!(type_def.fields[0].name, "FLAT");
    assert_eq!(type_def.fields[0].reserved_index, 0);
    assert_eq!(type_def.fields[1].name, "ROUND");
    assert_eq!(type_def.fields[1].reserved_index, 1);
    assert_eq!(type_def.fields[2].name, "POINTED");
    assert_eq!(type_def.fields[2].reserved_index, 2);

    // Check struct Color
    let color_def = &schema.definitions[1];
    assert_eq!(color_def.kind, DefinitionKind::Struct);
    assert_eq!(color_def.name, "Color");
    assert_eq!(color_def.fields.len(), 4);
    assert_eq!(color_def.fields[0].name, "red");
    assert_eq!(color_def.fields[0].type_.as_ref().unwrap(), "byte");
    assert_eq!(color_def.fields[0].is_array, false);
    assert_eq!(color_def.fields[0].reserved_index, 1);
    assert_eq!(color_def.fields[1].name, "green");
    assert_eq!(color_def.fields[1].type_.as_ref().unwrap(), "byte");
    assert_eq!(color_def.fields[1].is_array, false);
    assert_eq!(color_def.fields[1].reserved_index, 2);
    assert_eq!(color_def.fields[2].name, "blue");
    assert_eq!(color_def.fields[2].type_.as_ref().unwrap(), "byte");
    assert_eq!(color_def.fields[2].is_array, false);
    assert_eq!(color_def.fields[2].reserved_index, 3);
    assert_eq!(color_def.fields[3].name, "alpha");
    assert_eq!(color_def.fields[3].type_.as_ref().unwrap(), "byte");
    assert_eq!(color_def.fields[3].is_array, false);
    assert_eq!(color_def.fields[3].reserved_index, 4);

    // Check message Example
    let message_def = &schema.definitions[2];
    assert_eq!(message_def.kind, DefinitionKind::Message);
    assert_eq!(message_def.name, "Example");
    assert_eq!(message_def.fields.len(), 3);
    assert_eq!(message_def.fields[0].name, "clientID");
    assert_eq!(message_def.fields[0].type_.as_ref().unwrap(), "uint");
    assert_eq!(message_def.fields[0].is_array, false);
    assert_eq!(message_def.fields[0].reserved_index, 1);

    assert_eq!(message_def.fields[1].name, "type");
    assert_eq!(message_def.fields[1].type_.as_ref().unwrap(), "Type");
    assert_eq!(message_def.fields[1].is_array, false);
    assert_eq!(message_def.fields[1].reserved_index, 2);

    assert_eq!(message_def.fields[2].name, "colors");
    assert_eq!(message_def.fields[2].type_.as_ref().unwrap(), "Color");
    assert_eq!(message_def.fields[2].is_array, true);
    assert_eq!(message_def.fields[2].reserved_index, 3);

    let rust_code = compile_schema_to_rust(&schema);
    println!("Generated Rust code:\n{}", rust_code);
}
