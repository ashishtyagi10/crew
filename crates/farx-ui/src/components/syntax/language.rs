//! Language enum, extension-based detection, and comment prefix table.

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

    pub(super) fn comment_prefix(&self) -> &str {
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
