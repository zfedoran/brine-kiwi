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

