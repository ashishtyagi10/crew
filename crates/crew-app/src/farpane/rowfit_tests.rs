use super::fit;
use crate::chatwidth::str_w;

#[test]
fn a_file_keeps_its_size_and_marks_the_name() {
    let (name, pad) = fit("📄 a-really-long-filename.tar.gz".into(), "1.2M", 20);
    assert!(name.ends_with('\u{2026}'), "{name:?}");
    assert_eq!(str_w(&name) + pad + str_w("1.2M"), 20);
}

/// Directories used to slip past the guard and be hard-cut at the edge.
#[test]
fn a_directory_marks_its_cut_too() {
    let (name, pad) = fit("📁 a-really-long-project-directory-name/".into(), "", 20);
    assert!(name.ends_with('\u{2026}'), "{name:?}");
    assert_eq!(str_w(&name), 20, "{name:?}");
    assert_eq!(pad, 0);
}

#[test]
fn a_row_that_fits_is_left_alone_and_padded() {
    let (name, pad) = fit("📄 notes.md".into(), "3.1K", 30);
    assert_eq!(name, "📄 notes.md");
    assert_eq!(str_w(&name) + pad + 4, 30);
}

/// Width, not chars: CJK names count two per glyph.
#[test]
fn width_is_display_columns() {
    let (name, _) = fit("📁 日本語のフォルダの名前がとても長い/".into(), "", 16);
    assert!(str_w(&name) <= 16, "{name:?}");
    assert!(name.ends_with('\u{2026}'));
}
