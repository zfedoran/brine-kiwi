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

