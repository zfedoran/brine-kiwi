// example/src/main.rs

mod generated;

use std::collections::HashMap;
use brine_kiwi::*;

// Bring the generated types into scope:
use generated::{Color, Example, Type};

fn main() -> Result<(), KiwiError> {

    // Manually construct a `Value::Object("Example", fields)` that matches the schema.
    //
    // Build the inner field‐map: HashMap<&'static str, Value>
    let mut example_fields: HashMap<&'static str, Value> = HashMap::new();

    // 1) "clientID": 123
    example_fields.insert("clientID", Value::UInt(123));

    // 2) "type": Enum("Type", "ROUND")
    example_fields.insert("type", Value::Enum("Type", "ROUND"));

    // 3) "colors": an array of two Color‐objects
    let mut c1: HashMap<&'static str, Value> = HashMap::new();
    c1.insert("red",   Value::Byte(10));
    c1.insert("green", Value::Byte(20));
    c1.insert("blue",  Value::Byte(30));
    c1.insert("alpha", Value::Byte(255));
    let color1 = Value::Object("Color", c1);

    let mut c2: HashMap<&'static str, Value> = HashMap::new();
    c2.insert("red",   Value::Byte(200));
    c2.insert("green", Value::Byte(100));
    c2.insert("blue",  Value::Byte(50));
    c2.insert("alpha", Value::Byte(128));
    let color2 = Value::Object("Color", c2);

    example_fields.insert("colors", Value::Array(vec![color1, color2]));

    // Wrap the top‐level object as "Example"
    let v = Value::Object("Example", example_fields);

    // Now use the generated `Example::from_kiwi(&v)`:
    let example: Example = Example::from_kiwi(&v)?;

    // Because Example is a "message", its fields are `Option<…>`.
    let client_id = example.client_id.unwrap_or_default();
    let typ       = example.type_.unwrap_or(Type::Flat);
    let colors: Vec<Color> = example.colors.unwrap_or_default();

    println!("clientID = {}", client_id);
    println!("type    = {:?}", typ);
    println!("colors.len() = {}", colors.len());

    for (i, c) in colors.iter().enumerate() {
        println!(
            "  Color[{}] = (r={}, g={}, b={}, a={})",
            i, c.red, c.green, c.blue, c.alpha
        );
    }

    Ok(())
}
