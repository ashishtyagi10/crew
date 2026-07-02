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
mod tests {
    use super::*;

    fn file(name: &str) -> Entry {
        Entry {
            name: name.into(),
            is_dir: false,
            is_parent: false,
            size: 1,
        }
    }

    #[test]
    fn directories_and_parent_get_folder_glyphs() {
        let dir = Entry {
            name: "src".into(),
            is_dir: true,
            is_parent: false,
            size: 0,
        };
        let parent = Entry {
            name: "..".into(),
            is_dir: true,
            is_parent: true,
            size: 0,
        };
        assert_eq!(icon(&dir), '\u{f07b}');
        assert_eq!(icon(&parent), '\u{f062}');
    }

    #[test]
    fn extensions_map_to_type_glyphs_case_insensitively() {
        assert_eq!(icon(&file("main.rs")), '\u{e7a8}');
        assert_eq!(icon(&file("README.MD")), '\u{f48a}');
        assert_eq!(icon(&file("logo.png")), '\u{f1c5}');
        assert_eq!(icon(&file("bundle.tar")), '\u{f1c6}');
        assert_eq!(icon(&file("Cargo.toml")), '\u{f013}');
        assert_eq!(icon(&file("run.sh")), '\u{f489}');
    }

    #[test]
    fn unknown_and_extensionless_fall_back_to_the_generic_file() {
        assert_eq!(icon(&file("data.xyz")), '\u{f15b}');
        assert_eq!(icon(&file("Makefile")), '\u{f15b}');
        assert_eq!(icon(&file(".gitignore")), '\u{f15b}'); // leading dot ≠ extension
    }
}
