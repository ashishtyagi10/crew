//! Per-language tables for the fence lexer: comment openers, keyword sets,
//! and which languages carry capitalised types, `@decorators` or `#[attrs]`.
//! Split out of `syntax.rs`, which is the scanner and had no room left for
//! the data it scans with.

/// Line-comment openers per language, longest first so `///` is not mistaken
/// for `/` and `#!` is not mistaken for `#`.
pub(super) fn comment_starts(lang: &str) -> &'static [&'static str] {
    match lang {
        "python" | "py" | "sh" | "bash" | "zsh" | "shell" | "ruby" | "rb" | "yaml" | "yml"
        | "toml" | "makefile" | "dockerfile" | "r" | "perl" => &["#"],
        "sql" | "lua" | "haskell" | "hs" => &["--"],
        "lisp" | "clojure" | "clj" => &[";"],
        "vim" => &["\""],
        // C-family and everything unrecognised: `//` is the safest guess, and
        // a language without it simply gets no comment colouring rather than
        // a wrong one.
        _ => &["//"],
    }
}

/// Keywords per language. Small on purpose — the control-flow and declaration
/// words are what the eye uses to find structure; a complete list of every
/// reserved word would colour half the line and distinguish nothing.
pub(super) fn keywords(lang: &str) -> &'static [&'static str] {
    const RUST: &[&str] = &[
        "fn", "let", "mut", "const", "struct", "enum", "impl", "trait", "pub", "use", "mod",
        "match", "if", "else", "for", "while", "loop", "return", "self", "Self", "where", "async",
        "await", "move", "ref", "dyn", "as", "in", "break", "continue", "type", "static", "unsafe",
    ];
    const PY: &[&str] = &[
        "def", "class", "import", "from", "return", "if", "elif", "else", "for", "while", "try",
        "except", "finally", "with", "as", "lambda", "yield", "pass", "raise", "in", "not", "and",
        "or", "None", "True", "False", "async", "await", "global", "assert",
    ];
    const JS: &[&str] = &[
        "function",
        "const",
        "let",
        "var",
        "class",
        "extends",
        "return",
        "if",
        "else",
        "for",
        "while",
        "switch",
        "case",
        "break",
        "continue",
        "new",
        "this",
        "import",
        "export",
        "default",
        "async",
        "await",
        "try",
        "catch",
        "finally",
        "throw",
        "typeof",
        "interface",
        "type",
        "enum",
        "public",
        "private",
        "readonly",
        "null",
        "undefined",
    ];
    const GO: &[&str] = &[
        "func",
        "var",
        "const",
        "type",
        "struct",
        "interface",
        "package",
        "import",
        "return",
        "if",
        "else",
        "for",
        "range",
        "switch",
        "case",
        "defer",
        "go",
        "chan",
        "map",
        "nil",
    ];
    const SH: &[&str] = &[
        "if", "then", "else", "elif", "fi", "for", "in", "do", "done", "while", "case", "esac",
        "function", "return", "export", "local", "echo", "set",
    ];
    match lang {
        "rust" | "rs" => RUST,
        "python" | "py" => PY,
        "js" | "javascript" | "ts" | "typescript" | "tsx" | "jsx" => JS,
        "go" | "golang" => GO,
        "sh" | "bash" | "zsh" | "shell" | "console" => SH,
        // An unlabelled or unknown fence gets the C-family words, which
        // overlap heavily with most curly-brace languages. Wrong keywords are
        // cheap here: a missed word reads as plain code, which is the default.
        _ => JS,
    }
}

/// Languages where a capitalised identifier is, by convention, a type: Rust,
/// Go, TypeScript and Python all write `Foo` for a type and `foo` for a
/// value. Not shell (a capitalised word is a variable) and not an unknown
/// fence, where the guess would colour half of a prose-like line.
pub(super) fn has_types(lang: &str) -> bool {
    matches!(
        lang,
        "rust" | "rs" | "go" | "golang" | "ts" | "typescript" | "tsx" | "python" | "py"
    )
}

/// How a language spells an attribute, if it has one the lexer claims.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum AttrStyle {
    /// Rust's `#[derive(Debug)]` / `#![allow(..)]`, claimed to the closing `]`.
    Bracket,
    /// Python's and TypeScript's `@decorator`, claimed through the dotted name.
    Decorator,
    None,
}

pub(super) fn attr_style(lang: &str) -> AttrStyle {
    match lang {
        "rust" | "rs" => AttrStyle::Bracket,
        "python" | "py" | "ts" | "typescript" | "tsx" | "js" | "javascript" | "jsx" => {
            AttrStyle::Decorator
        }
        _ => AttrStyle::None,
    }
}
