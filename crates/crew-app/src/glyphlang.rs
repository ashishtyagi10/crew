//! The dev-icon for a fence language — the one table [`crate::glyphs`] keys
//! on a string rather than an enum variant. Split out of `glyphs` for the
//! line cap, along the line between the marks the pane draws and the
//! languages it knows an icon for.

/// The icon for `lang`, keyed on the info string's first word (```rust,ignore
/// and ```sh title=… still find theirs), case-insensitive. Anything unknown
/// gets the generic code icon, so a fence is never iconless on a Nerd Font.
pub(crate) fn lang_nerd(lang: &str) -> &'static str {
    match key(lang).as_str() {
        "rust" | "rs" => "\u{e7a8}",                               // nf-dev-rust
        "python" | "py" => "\u{e606}",                             // nf-seti-python
        "js" | "javascript" | "jsx" => "\u{e74e}",                 // nf-dev-javascript
        "ts" | "typescript" | "tsx" => "\u{e628}",                 // nf-seti-typescript
        "go" | "golang" => "\u{e626}",                             // nf-seti-go
        "sh" | "bash" | "zsh" | "shell" | "console" => "\u{e795}", // nf-dev-terminal
        "toml" => "\u{e6b2}",                                      // nf-seti-toml
        "yaml" | "yml" => "\u{e6a8}",                              // nf-seti-yml
        "json" => "\u{e60b}",                                      // nf-seti-json
        "md" | "markdown" => "\u{f48a}",                           // nf-oct-markdown
        "sql" => "\u{e706}",                                       // nf-dev-database
        "diff" | "patch" => "\u{f440}",                            // nf-oct-diff
        _ => "\u{f121}",                                           // nf-fa-code
    }
}

/// The language key of a fence info string: its first word, lower-cased.
/// `lang_nerd` and the badge hue (`chathue::lang_hue`) key on the same
/// word, so an icon and a colour can never disagree about what a fence is.
pub(crate) fn key(lang: &str) -> String {
    lang.split(|c: char| c == ',' || c.is_whitespace())
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}
