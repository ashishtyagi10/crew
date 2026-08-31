//! File-type icons for the Far panels: a Nerd Font glyph per entry, chosen by
//! extension (directories and the parent row get folder/up glyphs). These are
//! Private-Use-Area codepoints — they render as the intended dev-icons only
//! when a Nerd Font is the active crew font, and as tofu otherwise (an
//! accepted trade-off; every other font shows a placeholder box).
use super::Entry;

/// The Nerd Font glyph for `entry`: folder/parent glyphs for directories,
/// else an extension-based file glyph with a generic fallback.
pub(crate) fn icon(entry: &Entry) -> char {
    if entry.is_parent {
        return '\u{f062}'; // nf-fa-arrow_up
    }
    if entry.is_dir {
        return '\u{f07b}'; // nf-fa-folder
    }
    let ext = entry
        .name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "rs" => '\u{e7a8}',                                        // rust
        "md" | "markdown" => '\u{f48a}',                           // markdown
        "py" => '\u{e606}',                                        // python
        "js" | "mjs" | "cjs" | "ts" | "tsx" | "jsx" => '\u{e74e}', // js/ts
        "html" | "htm" => '\u{f13b}',                              // html5
        "css" | "scss" | "sass" => '\u{f13c}',                     // css3
        "json" | "toml" | "yaml" | "yml" | "ini" | "cfg" | "conf" | "lock" => '\u{f013}', // gear/config
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" | "bmp" => '\u{f1c5}',    // image
        "zip" | "tar" | "gz" | "xz" | "bz2" | "7z" | "rar" => '\u{f1c6}',                 // archive
        "sh" | "bash" | "zsh" | "fish" => '\u{f489}', // terminal
        "txt" | "log" | "text" => '\u{f0f6}',         // text file
        _ => '\u{f15b}',                              // generic file
    }
}

#[cfg(test)]
#[path = "icons_tests.rs"]
mod tests;
