use crate::{
    tokenizer::Token,
    types::{Definition, DefinitionKind, Field, Schema},
    utils::{error, quote},
    error::KiwiError,
};
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref IDENTIFIER:       Regex = Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").unwrap();
    static ref EQUALS:           Regex = Regex::new(r"^=$").unwrap();
    static ref SEMICOLON:        Regex = Regex::new(r"^;$").unwrap();
    static ref INTEGER:          Regex = Regex::new(r"^-?\d+$").unwrap();
    static ref LEFT_BRACE:       Regex = Regex::new(r"^\{$").unwrap();
    static ref RIGHT_BRACE:      Regex = Regex::new(r"^\}$").unwrap();
    static ref ARRAY_TOKEN:      Regex = Regex::new(r"^\[\]$").unwrap();
    static ref ENUM_KEYWORD:     Regex = Regex::new(r"^enum$").unwrap();
    static ref STRUCT_KEYWORD:   Regex = Regex::new(r"^struct$").unwrap();
    static ref MESSAGE_KEYWORD:  Regex = Regex::new(r"^message$").unwrap();
    static ref PACKAGE_KEYWORD:  Regex = Regex::new(r"^package$").unwrap();
    static ref DEPRECATED_TOKEN: Regex = Regex::new(r"^\[deprecated\]$").unwrap();
    static ref EOF:              Regex = Regex::new(r"^$").unwrap();
}

/// Now returns `Result<Schema, KiwiError>`.
pub fn parse_schema(tokens: &[Token]) -> Result<Schema, KiwiError> {
    let mut definitions  = Vec::new();
    let mut package_text = None;
    let mut index        = 0;

    fn current_token<'a>(tokens: &'a [Token], index: usize) -> &'a Token {
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

    fn expect(tokens: &[Token], index: &mut usize, test: &Regex, expected: &str) -> Result<(), KiwiError> {
        if !eat(tokens, index, test) {
            let tok = current_token(tokens, *index);
            return Err(error(
                &format!("Expected {} but found {}", expected, quote(&tok.text)),
                tok.line,
                tok.column,
            ));
        }
        Ok(())
    }

    fn unexpected_token(tokens: &[Token], index: &mut usize) -> KiwiError {
        let tok = current_token(tokens, *index);
        error(
            &format!("Unexpected token {}", quote(&tok.text)),
            tok.line,
            tok.column,
        )
    }

    // Handle package declaration
    if eat(tokens, &mut index, &PACKAGE_KEYWORD) {
        if index >= tokens.len() {
            return Err(error("Expected identifier after package", 0, 0));
        }
        let pkg_tok = current_token(tokens, index);
        expect(tokens, &mut index, &IDENTIFIER, "identifier")?;
        package_text = Some(pkg_tok.text.clone());
        expect(tokens, &mut index, &SEMICOLON, "\";\"")?;
    }

    // Parse definitions one by one
    while index < tokens.len() && !eat(tokens, &mut index, &EOF) {
        let kind = if eat(tokens, &mut index, &ENUM_KEYWORD) {
            DefinitionKind::Enum
        } else if eat(tokens, &mut index, &STRUCT_KEYWORD) {
            DefinitionKind::Struct
        } else if eat(tokens, &mut index, &MESSAGE_KEYWORD) {
            DefinitionKind::Message
        } else {
            return Err(unexpected_token(tokens, &mut index));
        };

        // Definition name
        let name_tok = current_token(tokens, index);
        expect(tokens, &mut index, &IDENTIFIER, "identifier")?;
        expect(tokens, &mut index, &LEFT_BRACE, "\"{\"")?;

        // Collect fields
        let mut fields = Vec::new();
        while !eat(tokens, &mut index, &RIGHT_BRACE) {
            let mut type_opt     = None;
            let mut is_array     = false;
            let mut is_deprecated = false;

            if kind != DefinitionKind::Enum {
                // Read the type token
                let t_tok = current_token(tokens, index);
                expect(tokens, &mut index, &IDENTIFIER, "identifier")?;
                if eat(tokens, &mut index, &ARRAY_TOKEN) {
                    is_array = true;
                }
                type_opt = Some(t_tok.text.clone());
            }

            // Field name
            let f_tok = current_token(tokens, index);
            expect(tokens, &mut index, &IDENTIFIER, "identifier")?;

            // Value (either explicit or auto‐increment for structs)
            let value = if kind != DefinitionKind::Struct {
                expect(tokens, &mut index, &EQUALS, "\"=\"")?;
                let v_tok = current_token(tokens, index);
                expect(tokens, &mut index, &INTEGER, "integer")?;
                v_tok.text.parse::<i32>().map_err(|_| {
                    error(
                        &format!("Invalid integer {}", quote(&v_tok.text)),
                        v_tok.line,
                        v_tok.column,
                    )
                })?
            } else {
                // For structs, assign in‐order values
                fields.len() as i32 + 1
            };

            // Deprecated?
            if eat(tokens, &mut index, &DEPRECATED_TOKEN) {
                if kind != DefinitionKind::Message {
                    let deprecated = current_token(tokens, index - 1);
                    return Err(error("Cannot deprecate this field", deprecated.line, deprecated.column));
                }
                is_deprecated = true;
            }

            expect(tokens, &mut index, &SEMICOLON, "\";\"")?;

            let final_value = if kind != DefinitionKind::Struct {
                value
            } else {
                fields.len() as i32 + 1
            };

            fields.push(Field {
                name:           f_tok.text.clone(),
                line:           f_tok.line,
                column:         f_tok.column,
                type_:          type_opt.clone(),
                is_array,
                is_deprecated,
                reserved_index: final_value,
            });
        }

        definitions.push(Definition {
            name:    name_tok.text.clone(),
            line:    name_tok.line,
            column:  name_tok.column,
            kind,
            fields,
        });
    }

    Ok(Schema {
        package:    package_text,
        definitions,
    })
}
