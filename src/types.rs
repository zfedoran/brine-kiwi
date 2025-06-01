#[derive(Debug, PartialEq)]
pub struct Schema {
    pub package:     Option<String>,
    pub definitions: Vec<Definition>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DefinitionKind {
    Enum    = 0,
    Struct  = 1,
    Message = 2,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name:           String,
    pub line:           usize,
    pub column:         usize,
    pub type_:          Option<String>,
    pub is_array:       bool,
    pub is_deprecated:  bool,
    pub reserved_index: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    pub name:    String,
    pub line:    usize,
    pub column:  usize,
    pub kind:    DefinitionKind,
    pub fields:  Vec<Field>,
}
