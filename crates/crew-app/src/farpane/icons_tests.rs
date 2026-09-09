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
    let _on = crate::glyphs::force(true);
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
    let _on = crate::glyphs::force(true);
    assert_eq!(icon(&file("main.rs")), '\u{e7a8}');
    assert_eq!(icon(&file("README.MD")), '\u{f48a}');
    assert_eq!(icon(&file("logo.png")), '\u{f1c5}');
    assert_eq!(icon(&file("bundle.tar")), '\u{f1c6}');
    assert_eq!(icon(&file("Cargo.toml")), '\u{f013}');
    assert_eq!(icon(&file("run.sh")), '\u{f489}');
}

#[test]
fn unknown_and_extensionless_fall_back_to_the_generic_file() {
    let _on = crate::glyphs::force(true);
    assert_eq!(icon(&file("data.xyz")), '\u{f15b}');
    assert_eq!(icon(&file("Makefile")), '\u{f15b}');
    assert_eq!(icon(&file(".gitignore")), '\u{f15b}'); // leading dot ≠ extension
}

#[test]
fn without_the_icon_set_every_entry_gets_a_plain_unicode_mark() {
    let _off = crate::glyphs::force(false);
    let dir = Entry {
        name: "src".into(),
        is_dir: true,
        is_parent: false,
        size: 0,
    };
    assert_eq!(icon(&dir), '\u{25b8}'); // ▸ — not the PUA folder
    assert_eq!(icon(&file("main.rs")), '\u{00b7}'); // · — not the PUA rust glyph
    assert_eq!(icon(&file("data.xyz")), '\u{00b7}');
}
