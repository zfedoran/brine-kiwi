use regex::Regex;
use lazy_static::lazy_static;
use crate::utils::{quote, error};
use crate::error::KiwiError;

lazy_static! {
    pub static ref TOKEN_REGEX:    Regex = Regex::new(r"((?:-|\b)\d+\b|[=;{}]|\[\]|\[deprecated\]|\b[A-Za-z_][A-Za-z0-9_]*\b|//.*|\s+)").unwrap();
    pub static ref WHITESPACE_RX:  Regex = Regex::new(r"^(//.*|\s+)$").unwrap();
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub text:   String,
    pub line:   usize,
    pub column: usize,
}

/// Now returns `Result<Vec<Token>, KiwiError>`.
pub fn tokenize_schema(text: &str) -> Result<Vec<Token>, KiwiError> {
    let mut tokens = Vec::new();
    let mut line = 1;
    let mut column = 1;
    let mut last_end = 0;

    for mat in TOKEN_REGEX.find_iter(text) {
        let start = mat.start();
        let end   = mat.end();
        let part  = mat.as_str();

        if start > last_end {
            // Unexpected text between last_end and start
            let unexpected = &text[last_end..start];
            return Err(error(
                &format!("Syntax error: {}", quote(unexpected)),
                line,
                column,
            ));
        }

        if !WHITESPACE_RX.is_match(part) && !part.starts_with("//") {
            tokens.push(Token {
                text:   part.to_string(),
                line,
                column,
            });
        }

        // Update line/column
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
        return Err(error(
            &format!("Syntax error: {}", quote(unexpected)),
            line,
            column,
        ));
    }

    // Append EOF token
    tokens.push(Token {
        text:   "".to_string(),
        line,
        column,
    });
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let input = "int x = 10;";
        let expected = vec![
            Token { text: "int".into(), line: 1, column: 1 },
            Token { text: "x".into(),   line: 1, column: 5 },
            Token { text: "=".into(),   line: 1, column: 7 },
            Token { text: "10".into(),  line: 1, column: 9 },
            Token { text: ";".into(),   line: 1, column: 11 },
            Token { text: "".into(),    line: 1, column: 12 },
        ];
        let got = tokenize_schema(input).unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_tokenize_with_deprecated_tag() {
        let input = "[deprecated]";
        let expected = vec![
            Token { text: "[deprecated]".into(), line: 1, column: 1 },
            Token { text: "".into(),             line: 1, column: 13 },
        ];
        let got = tokenize_schema(input).unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_tokenize_reserved_names() {
        let input = "ByteBuffer package";
        let expected = vec![
            Token { text: "ByteBuffer".into(), line: 1, column: 1 },
            Token { text: "package".into(),    line: 1, column: 12 },
            Token { text: "".into(),           line: 1, column: 19 },
        ];
        let got = tokenize_schema(input).unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_tokenize_unexpected_text() {
        let input = "int x = 10 @";
        let err = tokenize_schema(input).unwrap_err();
        assert!(
            matches!(err, KiwiError::ParseError { .. }),
            "expected a ParseError but got {:?}",
            err
        );
    }
}
