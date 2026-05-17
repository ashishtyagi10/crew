//! Per-language keyword and control-flow lookup.

use super::keywords_data as kw;
use super::language::Language;

impl Language {
    pub(super) fn keywords(&self) -> &[&str] {
        match self {
            Self::Rust => kw::RUST,
            Self::Python => kw::PYTHON,
            Self::JavaScript | Self::TypeScript => kw::JS_TS,
            Self::Go => kw::GO,
            Self::C | Self::Cpp => kw::C_CPP,
            Self::Java => kw::JAVA,
            Self::Ruby => kw::RUBY,
            Self::Shell => kw::SHELL,
            _ => &[],
        }
    }

    pub(super) fn control_flow(&self) -> &[&str] {
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
}
