use serde_json;

pub fn quote(text: &str) -> String {
    serde_json::to_string(text).unwrap()
}

pub fn error(msg: &str, line: usize, column: usize) -> ! {
    panic!("Error at line {}, column {}: {}", line, column, msg);
}
