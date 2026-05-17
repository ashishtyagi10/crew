//! Per-language keyword constants. Kept separate from the dispatcher so the
//! dispatcher stays well under the line cap after rustfmt expands arrays.
//!
//! `rustfmt::skip` is applied per-item to keep these tables compact.

#[rustfmt::skip]
pub(super) const RUST: &[&str] = &[
    "fn", "let", "mut", "const", "static", "pub", "mod", "use", "crate", "self", "super",
    "struct", "enum", "impl", "trait", "type", "where", "for", "in", "loop", "while", "if",
    "else", "match", "return", "break", "continue", "as", "ref", "move", "async", "await",
    "dyn", "unsafe", "extern", "true", "false", "Some", "None", "Ok", "Err", "Self",
];

#[rustfmt::skip]
pub(super) const PYTHON: &[&str] = &[
    "def", "class", "return", "if", "elif", "else", "for", "while", "in", "import", "from",
    "as", "try", "except", "finally", "raise", "with", "yield", "lambda", "pass", "break",
    "continue", "and", "or", "not", "is", "None", "True", "False", "self", "async", "await",
    "global",
];

#[rustfmt::skip]
pub(super) const JS_TS: &[&str] = &[
    "function", "const", "let", "var", "return", "if", "else", "for", "while", "do", "switch",
    "case", "break", "continue", "new", "delete", "typeof", "instanceof", "in", "of", "class",
    "extends", "super", "this", "import", "export", "from", "default", "async", "await",
    "yield", "try", "catch", "finally", "throw", "true", "false", "null", "undefined",
    "interface", "type", "enum", "implements", "abstract", "readonly",
];

#[rustfmt::skip]
pub(super) const GO: &[&str] = &[
    "func", "package", "import", "var", "const", "type", "struct", "interface", "map", "chan",
    "go", "select", "case", "default", "if", "else", "for", "range", "switch", "return",
    "break", "continue", "defer", "fallthrough", "goto", "true", "false", "nil", "make", "new",
    "append", "len", "cap",
];

#[rustfmt::skip]
pub(super) const C_CPP: &[&str] = &[
    "int", "char", "float", "double", "void", "long", "short", "unsigned", "signed", "const",
    "static", "extern", "auto", "register", "volatile", "if", "else", "for", "while", "do",
    "switch", "case", "default", "break", "continue", "return", "goto", "sizeof", "typedef",
    "struct", "union", "enum", "NULL", "true", "false",
    // C++ extras
    "class", "public", "private", "protected", "virtual", "override", "template", "typename",
    "namespace", "using", "new", "delete", "try", "catch", "throw", "nullptr", "this", "auto",
];

#[rustfmt::skip]
pub(super) const JAVA: &[&str] = &[
    "public", "private", "protected", "static", "final", "abstract", "class", "interface",
    "extends", "implements", "new", "this", "super", "if", "else", "for", "while", "do",
    "switch", "case", "default", "break", "continue", "return", "throw", "try", "catch",
    "finally", "import", "package", "void", "int", "long", "double", "float", "boolean",
    "char", "byte", "short", "true", "false", "null",
];

#[rustfmt::skip]
pub(super) const RUBY: &[&str] = &[
    "def", "end", "class", "module", "if", "elsif", "else", "unless", "while", "until", "for",
    "in", "do", "begin", "rescue", "ensure", "raise", "return", "yield", "block_given?",
    "self", "super", "true", "false", "nil", "and", "or", "not", "require", "include",
    "attr_reader", "attr_writer", "attr_accessor", "puts", "print",
];

#[rustfmt::skip]
pub(super) const SHELL: &[&str] = &[
    "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac", "in",
    "function", "return", "exit", "echo", "export", "local", "readonly", "set", "unset",
    "shift", "source", "true", "false",
];
