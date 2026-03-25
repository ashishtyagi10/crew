//! Simple regex-free syntax highlighter for common file types.
//! Produces colored Spans for each line based on file extension.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

const BG: Color = Color::Rgb(22, 22, 26);
const BG_CURSOR: Color = Color::Indexed(236);

// Color palette — rich, distinct colors
const C_KEYWORD: Color = Color::Rgb(220, 170, 60); // amber — keywords (if, for, fn)
const C_CONTROL: Color = Color::Rgb(230, 120, 100); // coral — control flow (return, break)
const C_STRING: Color = Color::Rgb(120, 195, 90); // green — strings
const C_COMMENT: Color = Color::Rgb(95, 95, 120); // muted gray-blue — comments
const C_NUMBER: Color = Color::Rgb(235, 145, 70); // orange — numbers
const C_TYPE: Color = Color::Rgb(90, 185, 165); // teal — types/classes
const C_BUILTIN: Color = Color::Rgb(130, 170, 220); // soft blue — builtin types/fns
const C_FUNC: Color = Color::Rgb(200, 180, 130); // warm sand — function calls
const C_OPERATOR: Color = Color::Rgb(195, 125, 175); // pink — operators
const C_MACRO: Color = Color::Rgb(180, 150, 220); // lavender — macros/attributes/decorators
const C_LIFETIME: Color = Color::Rgb(220, 140, 160); // rose — lifetimes ('a)
const C_PLAIN: Color = Color::Rgb(192, 188, 180); // warm gray — default text
const C_PUNCT: Color = Color::Rgb(120, 118, 112); // dim — punctuation/brackets
const C_SPECIAL: Color = Color::Rgb(160, 200, 230); // light blue — self/this/super

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    C,
    Cpp,
    Java,
    Ruby,
    Shell,
    Toml,
    Yaml,
    Json,
    Markdown,
    Html,
    Css,
    Sql,
    Unknown,
}

impl Language {
    pub fn from_extension(ext: Option<&str>) -> Self {
        match ext {
            Some("rs") => Self::Rust,
            Some("py" | "pyw") => Self::Python,
            Some("js" | "jsx" | "mjs" | "cjs") => Self::JavaScript,
            Some("ts" | "tsx") => Self::TypeScript,
            Some("go") => Self::Go,
            Some("c" | "h") => Self::C,
            Some("cpp" | "cc" | "cxx" | "hpp" | "hh") => Self::Cpp,
            Some("java") => Self::Java,
            Some("rb") => Self::Ruby,
            Some("sh" | "bash" | "zsh" | "fish") => Self::Shell,
            Some("toml") => Self::Toml,
            Some("yaml" | "yml") => Self::Yaml,
            Some("json" | "jsonc") => Self::Json,
            Some("md" | "markdown") => Self::Markdown,
            Some("html" | "htm" | "xml" | "svg") => Self::Html,
            Some("css" | "scss" | "less") => Self::Css,
            Some("sql") => Self::Sql,
            _ => Self::Unknown,
        }
    }

    fn keywords(&self) -> &[&str] {
        match self {
            Self::Rust => &[
                "fn", "let", "mut", "const", "static", "pub", "mod", "use", "crate", "self",
                "super", "struct", "enum", "impl", "trait", "type", "where", "for", "in", "loop",
                "while", "if", "else", "match", "return", "break", "continue", "as", "ref", "move",
                "async", "await", "dyn", "unsafe", "extern", "true", "false", "Some", "None", "Ok",
                "Err", "Self",
            ],
            Self::Python => &[
                "def", "class", "return", "if", "elif", "else", "for", "while", "in", "import",
                "from", "as", "try", "except", "finally", "raise", "with", "yield", "lambda",
                "pass", "break", "continue", "and", "or", "not", "is", "None", "True", "False",
                "self", "async", "await", "global",
            ],
            Self::JavaScript | Self::TypeScript => &[
                "function",
                "const",
                "let",
                "var",
                "return",
                "if",
                "else",
                "for",
                "while",
                "do",
                "switch",
                "case",
                "break",
                "continue",
                "new",
                "delete",
                "typeof",
                "instanceof",
                "in",
                "of",
                "class",
                "extends",
                "super",
                "this",
                "import",
                "export",
                "from",
                "default",
                "async",
                "await",
                "yield",
                "try",
                "catch",
                "finally",
                "throw",
                "true",
                "false",
                "null",
                "undefined",
                "interface",
                "type",
                "enum",
                "implements",
                "abstract",
                "readonly",
            ],
            Self::Go => &[
                "func",
                "package",
                "import",
                "var",
                "const",
                "type",
                "struct",
                "interface",
                "map",
                "chan",
                "go",
                "select",
                "case",
                "default",
                "if",
                "else",
                "for",
                "range",
                "switch",
                "return",
                "break",
                "continue",
                "defer",
                "fallthrough",
                "goto",
                "true",
                "false",
                "nil",
                "make",
                "new",
                "append",
                "len",
                "cap",
            ],
            Self::C | Self::Cpp => &[
                "int",
                "char",
                "float",
                "double",
                "void",
                "long",
                "short",
                "unsigned",
                "signed",
                "const",
                "static",
                "extern",
                "auto",
                "register",
                "volatile",
                "if",
                "else",
                "for",
                "while",
                "do",
                "switch",
                "case",
                "default",
                "break",
                "continue",
                "return",
                "goto",
                "sizeof",
                "typedef",
                "struct",
                "union",
                "enum",
                "NULL",
                "true",
                "false",
                // C++ extras
                "class",
                "public",
                "private",
                "protected",
                "virtual",
                "override",
                "template",
                "typename",
                "namespace",
                "using",
                "new",
                "delete",
                "try",
                "catch",
                "throw",
                "nullptr",
                "this",
                "auto",
            ],
            Self::Java => &[
                "public",
                "private",
                "protected",
                "static",
                "final",
                "abstract",
                "class",
                "interface",
                "extends",
                "implements",
                "new",
                "this",
                "super",
                "if",
                "else",
                "for",
                "while",
                "do",
                "switch",
                "case",
                "default",
                "break",
                "continue",
                "return",
                "throw",
                "try",
                "catch",
                "finally",
                "import",
                "package",
                "void",
                "int",
                "long",
                "double",
                "float",
                "boolean",
                "char",
                "byte",
                "short",
                "true",
                "false",
                "null",
            ],
            Self::Ruby => &[
                "def",
                "end",
                "class",
                "module",
                "if",
                "elsif",
                "else",
                "unless",
                "while",
                "until",
                "for",
                "in",
                "do",
                "begin",
                "rescue",
                "ensure",
                "raise",
                "return",
                "yield",
                "block_given?",
                "self",
                "super",
                "true",
                "false",
                "nil",
                "and",
                "or",
                "not",
                "require",
                "include",
                "attr_reader",
                "attr_writer",
                "attr_accessor",
                "puts",
                "print",
            ],
            Self::Shell => &[
                "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac",
                "in", "function", "return", "exit", "echo", "export", "local", "readonly", "set",
                "unset", "shift", "source", "true", "false",
            ],
            _ => &[],
        }
    }

    fn control_flow(&self) -> &[&str] {
        match self {
            Self::Rust => &[
                "return", "break", "continue", "if", "else", "match", "loop", "while", "for",
            ],
            Self::Python => &[
                "return", "break", "continue", "if", "elif", "else", "for", "while", "raise",
                "yield",
            ],
            Self::JavaScript | Self::TypeScript => &[
                "return", "break", "continue", "if", "else", "for", "while", "throw", "yield",
                "switch",
            ],
            Self::Go => &[
                "return", "break", "continue", "if", "else", "for", "switch", "select", "goto",
                "defer",
            ],
            Self::C | Self::Cpp => &[
                "return", "break", "continue", "if", "else", "for", "while", "do", "switch", "goto",
            ],
            Self::Java => &[
                "return", "break", "continue", "if", "else", "for", "while", "do", "switch",
                "throw",
            ],
            Self::Ruby => &[
                "return", "if", "elsif", "else", "unless", "while", "until", "for", "raise",
                "yield",
            ],
            Self::Shell => &[
                "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "return", "exit",
            ],
            _ => &[],
        }
    }

    fn builtins(&self) -> &[&str] {
        match self {
            Self::Rust => &[
                "String",
                "Vec",
                "Option",
                "Result",
                "Box",
                "Rc",
                "Arc",
                "HashMap",
                "HashSet",
                "println!",
                "eprintln!",
                "format!",
                "vec!",
                "panic!",
                "todo!",
                "unimplemented!",
                "assert!",
                "assert_eq!",
                "dbg!",
                "write!",
                "writeln!",
                "u8",
                "u16",
                "u32",
                "u64",
                "u128",
                "usize",
                "i8",
                "i16",
                "i32",
                "i64",
                "i128",
                "isize",
                "f32",
                "f64",
                "bool",
                "str",
                "char",
                "Self",
            ],
            Self::Python => &[
                "print",
                "len",
                "range",
                "enumerate",
                "zip",
                "map",
                "filter",
                "sorted",
                "reversed",
                "list",
                "dict",
                "set",
                "tuple",
                "str",
                "int",
                "float",
                "bool",
                "bytes",
                "isinstance",
                "issubclass",
                "type",
                "super",
                "property",
                "staticmethod",
                "classmethod",
                "open",
                "input",
                "abs",
                "min",
                "max",
                "sum",
                "any",
                "all",
                "iter",
                "next",
            ],
            Self::JavaScript | Self::TypeScript => &[
                "console",
                "Math",
                "JSON",
                "Array",
                "Object",
                "Map",
                "Set",
                "Promise",
                "parseInt",
                "parseFloat",
                "isNaN",
                "setTimeout",
                "setInterval",
                "string",
                "number",
                "boolean",
                "void",
                "never",
                "any",
                "unknown",
                "require",
                "module",
                "exports",
                "process",
                "Buffer",
            ],
            Self::Go => &[
                "make",
                "new",
                "append",
                "len",
                "cap",
                "copy",
                "delete",
                "close",
                "panic",
                "recover",
                "print",
                "println",
                "error",
                "string",
                "byte",
                "rune",
                "int",
                "int8",
                "int16",
                "int32",
                "int64",
                "uint",
                "uint8",
                "uint16",
                "uint32",
                "uint64",
                "float32",
                "float64",
                "complex64",
                "complex128",
                "bool",
            ],
            _ => &[],
        }
    }

    fn special_idents(&self) -> &[&str] {
        match self {
            Self::Rust => &["self", "super", "crate", "Self"],
            Self::Python => &["self", "cls", "__init__", "__main__", "__name__"],
            Self::JavaScript | Self::TypeScript => {
                &["this", "super", "globalThis", "window", "document"]
            }
            Self::Go => &["nil"],
            Self::Java => &["this", "super"],
            Self::Ruby => &["self", "super"],
            _ => &[],
        }
    }

    fn comment_prefix(&self) -> &str {
        match self {
            Self::Rust
            | Self::Go
            | Self::C
            | Self::Cpp
            | Self::Java
            | Self::JavaScript
            | Self::TypeScript
            | Self::Css => "//",
            Self::Python | Self::Ruby | Self::Shell | Self::Toml | Self::Yaml => "#",
            Self::Html => "<!--",
            Self::Sql => "--",
            _ => "",
        }
    }
}

/// Highlight a single line of code, returning owned styled spans.
pub fn highlight_line(line: &str, lang: Language, bg: Color) -> Vec<Span<'static>> {
    if lang == Language::Unknown {
        return vec![Span::styled(
            line.to_string(),
            Style::default().fg(C_PLAIN).bg(bg),
        )];
    }

    let trimmed = line.trim_start();
    let comment_prefix = lang.comment_prefix();
    if !comment_prefix.is_empty() && trimmed.starts_with(comment_prefix) {
        return vec![Span::styled(
            line.to_string(),
            Style::default()
                .fg(C_COMMENT)
                .bg(bg)
                .add_modifier(Modifier::ITALIC),
        )];
    }

    if lang == Language::Markdown {
        return highlight_markdown_line(line, bg);
    }
    if lang == Language::Json {
        return highlight_json_line(line, bg);
    }

    let keywords = lang.keywords();
    let control = lang.control_flow();
    let builtins = lang.builtins();
    let specials = lang.special_idents();
    let mut spans = Vec::new();
    let mut chars = line.char_indices().peekable();

    while let Some(&(i, ch)) = chars.peek() {
        if ch == '"'
            || ch == '\''
            || (ch == '`' && matches!(lang, Language::JavaScript | Language::TypeScript))
        {
            let quote = ch;
            let start = i;
            chars.next();
            let mut escaped = false;
            while let Some(&(_, c)) = chars.peek() {
                chars.next();
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == quote {
                    break;
                }
            }
            let end = chars.peek().map(|&(i, _)| i).unwrap_or(line.len());
            spans.push(Span::styled(
                line[start..end].to_string(),
                Style::default().fg(C_STRING).bg(bg),
            ));
            continue;
        }

        if ch == '/'
            && matches!(
                lang,
                Language::Rust
                    | Language::Go
                    | Language::C
                    | Language::Cpp
                    | Language::Java
                    | Language::JavaScript
                    | Language::TypeScript
                    | Language::Css
            )
        {
            if line[i..].starts_with("//") {
                spans.push(Span::styled(
                    line[i..].to_string(),
                    Style::default()
                        .fg(C_COMMENT)
                        .bg(bg)
                        .add_modifier(Modifier::ITALIC),
                ));
                return spans;
            }
        }
        if ch == '#'
            && matches!(
                lang,
                Language::Python
                    | Language::Ruby
                    | Language::Shell
                    | Language::Toml
                    | Language::Yaml
            )
        {
            spans.push(Span::styled(
                line[i..].to_string(),
                Style::default()
                    .fg(C_COMMENT)
                    .bg(bg)
                    .add_modifier(Modifier::ITALIC),
            ));
            return spans;
        }

        if ch.is_ascii_digit()
            && (i == 0
                || !line
                    .as_bytes()
                    .get(i.wrapping_sub(1))
                    .map(|b| b.is_ascii_alphanumeric() || *b == b'_')
                    .unwrap_or(false))
        {
            let start = i;
            while let Some(&(_, c)) = chars.peek() {
                if c.is_ascii_alphanumeric()
                    || c == '.'
                    || c == '_'
                    || c == 'x'
                    || c == 'b'
                    || c == 'o'
                {
                    chars.next();
                } else {
                    break;
                }
            }
            let end = chars.peek().map(|&(i, _)| i).unwrap_or(line.len());
            spans.push(Span::styled(
                line[start..end].to_string(),
                Style::default().fg(C_NUMBER).bg(bg),
            ));
            continue;
        }

        // Rust lifetimes: 'a, 'static, 'static
        if ch == '\'' && lang == Language::Rust {
            let start = i;
            chars.next(); // consume quote
            while let Some(&(_, c)) = chars.peek() {
                if c.is_ascii_alphanumeric() || c == '_' {
                    chars.next();
                } else {
                    break;
                }
            }
            let end = chars.peek().map(|&(i, _)| i).unwrap_or(line.len());
            if end > start + 1 {
                spans.push(Span::styled(
                    line[start..end].to_string(),
                    Style::default().fg(C_LIFETIME).bg(bg),
                ));
                continue;
            }
            // Single quote (char literal) — fall through to string handler
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(C_PLAIN).bg(bg),
            ));
            continue;
        }

        // Python/Ruby decorators: @decorator
        if ch == '@'
            && matches!(
                lang,
                Language::Python | Language::Java | Language::TypeScript
            )
        {
            let start = i;
            chars.next();
            while let Some(&(_, c)) = chars.peek() {
                if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
                    chars.next();
                } else {
                    break;
                }
            }
            let end = chars.peek().map(|&(i, _)| i).unwrap_or(line.len());
            spans.push(Span::styled(
                line[start..end].to_string(),
                Style::default().fg(C_MACRO).bg(bg),
            ));
            continue;
        }

        // Rust attributes
        if ch == '#' && lang == Language::Rust {
            let start = i;
            while let Some(&(_, c)) = chars.peek() {
                chars.next();
                if c == ']' {
                    break;
                }
            }
            let end = chars.peek().map(|&(i, _)| i).unwrap_or(line.len());
            spans.push(Span::styled(
                line[start..end].to_string(),
                Style::default().fg(C_MACRO).bg(bg),
            ));
            continue;
        }

        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            while let Some(&(_, c)) = chars.peek() {
                if c.is_ascii_alphanumeric() || c == '_' || c == '!' || c == '?' {
                    chars.next();
                } else {
                    break;
                }
            }
            let end = chars.peek().map(|&(i, _)| i).unwrap_or(line.len());
            let word = &line[start..end];

            let bare_word = word.trim_end_matches('!');
            let style = if control.contains(&bare_word) {
                // Control flow: coral, bold
                Style::default()
                    .fg(C_CONTROL)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD)
            } else if specials.contains(&bare_word) {
                // self/this/super: light blue
                Style::default()
                    .fg(C_SPECIAL)
                    .bg(bg)
                    .add_modifier(Modifier::ITALIC)
            } else if keywords.contains(&bare_word) {
                // Language keywords: amber, bold
                Style::default()
                    .fg(C_KEYWORD)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD)
            } else if builtins.contains(&word) {
                // Builtin types/functions: soft blue
                Style::default().fg(C_BUILTIN).bg(bg)
            } else if word.ends_with('!') && lang == Language::Rust {
                // Rust macro invocation: lavender
                Style::default().fg(C_MACRO).bg(bg)
            } else if word.starts_with("__") && word.ends_with("__") {
                // Python dunder methods: lavender
                Style::default().fg(C_MACRO).bg(bg)
            } else if word
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
            {
                // Type-like (starts with uppercase): teal
                Style::default().fg(C_TYPE).bg(bg)
            } else if chars.peek().map(|&(_, c)| c == '(').unwrap_or(false) {
                // Function call (followed by `(`): warm sand
                Style::default().fg(C_FUNC).bg(bg)
            } else {
                Style::default().fg(C_PLAIN).bg(bg)
            };
            spans.push(Span::styled(word.to_string(), style));
            continue;
        }

        if "=+-*/<>!&|^%~?:".contains(ch) {
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(C_OPERATOR).bg(bg),
            ));
            chars.next();
            continue;
        }

        if "{}[]();,.@".contains(ch) {
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(C_PUNCT).bg(bg),
            ));
            chars.next();
            continue;
        }

        spans.push(Span::styled(
            ch.to_string(),
            Style::default().fg(C_PLAIN).bg(bg),
        ));
        chars.next();
    }

    spans
}

fn highlight_markdown_line(line: &str, bg: Color) -> Vec<Span<'static>> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("# ") || trimmed.starts_with("## ") || trimmed.starts_with("### ") {
        return vec![Span::styled(
            line.to_string(),
            Style::default()
                .fg(C_KEYWORD)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        )];
    }
    if trimmed.starts_with("```") {
        return vec![Span::styled(
            line.to_string(),
            Style::default().fg(C_COMMENT).bg(bg),
        )];
    }
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        return vec![Span::styled(
            line.to_string(),
            Style::default().fg(C_TYPE).bg(bg),
        )];
    }
    if trimmed.starts_with("> ") {
        return vec![Span::styled(
            line.to_string(),
            Style::default()
                .fg(C_STRING)
                .bg(bg)
                .add_modifier(Modifier::ITALIC),
        )];
    }
    vec![Span::styled(
        line.to_string(),
        Style::default().fg(C_PLAIN).bg(bg),
    )]
}

fn highlight_json_line(line: &str, bg: Color) -> Vec<Span<'static>> {
    let trimmed = line.trim();
    if trimmed.starts_with('"') && trimmed.contains("\":") {
        if let Some(colon_pos) = line.find(':') {
            let key_part = &line[..colon_pos + 1];
            let val_part = &line[colon_pos + 1..];
            let val_trimmed = val_part.trim();
            let val_color = if val_trimmed.starts_with('"') {
                C_STRING
            } else if val_trimmed.starts_with(|c: char| c.is_ascii_digit() || c == '-') {
                C_NUMBER
            } else if val_trimmed == "true"
                || val_trimmed == "false"
                || val_trimmed == "null"
                || val_trimmed == "true,"
                || val_trimmed == "false,"
                || val_trimmed == "null,"
            {
                C_KEYWORD
            } else {
                C_PLAIN
            };
            return vec![
                Span::styled(key_part.to_string(), Style::default().fg(C_TYPE).bg(bg)),
                Span::styled(val_part.to_string(), Style::default().fg(val_color).bg(bg)),
            ];
        }
    }
    if trimmed.starts_with('"') {
        return vec![Span::styled(
            line.to_string(),
            Style::default().fg(C_STRING).bg(bg),
        )];
    }
    vec![Span::styled(
        line.to_string(),
        Style::default().fg(C_PUNCT).bg(bg),
    )]
}
