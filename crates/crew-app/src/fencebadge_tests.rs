use super::*;
use crate::chatbody::CardLine;
use crate::glyphs::force;
use crew_theme::contrast_ratio;

fn lines(text: &str, width: usize) -> Vec<CardLine> {
    crate::chatmd::map_lines(crate::md::render_chat(text, width), width, (9, 9, 9))
}

fn row_text(line: &CardLine) -> String {
    line.iter().map(|c| c.c).collect()
}

/// On main the language row was `  rust      `: the bare label, muted, on
/// the field. It is a badge now — capped, on the language's hue, the field
/// continuing past it.
#[test]
fn the_fence_header_is_a_capped_badge_on_the_languages_hue() {
    let _g = crate::app::theme_test_guard();
    let _off = force(false);
    let out = lines("```rust\nfn x() {}\n```", 40);
    let row = row_text(&out[0]);
    assert!(
        row.starts_with("  \u{2590} rust \u{258c}"),
        "not a badge: {row:?}"
    );
    let field = Some(crate::chatink::code_bg());
    let hue = crate::chathue::lang_hue("rust");
    assert_ne!(
        hue,
        crate::chatink::code_bg(),
        "the block stands off the field"
    );
    // indent, pad, cap, pad, r u s t, pad, cap, field…
    assert_eq!(out[0][1].bg, field, "the field's own pad");
    assert_eq!((out[0][2].fg, out[0][2].bg), (hue, field), "left cap");
    assert_eq!((out[0][9].fg, out[0][9].bg), (hue, field), "right cap");
    for cell in &out[0][3..9] {
        assert_eq!(cell.bg, Some(hue), "{:?} is on the block", cell.c);
        assert!(
            contrast_ratio(cell.fg, hue) >= crew_theme::contrast::text_floor(),
            "{:?}: {:?} on {hue:?}",
            cell.c,
            cell.fg
        );
    }
    assert!(out[0][4..8].iter().all(|c| c.bold), "the label is bold");
    assert!(
        out[0][10..].iter().all(|c| c.bg == field),
        "the field continues past the badge: {row:?}"
    );
    // The rows below are untouched: same field, same width.
    assert_eq!(row_text(&out[1]), "  fn x() {} ");
    assert!(out[1][1..].iter().all(|c| c.bg == field));
}

/// The language is read back off the label, past any icon.
#[test]
fn the_language_is_the_labels_first_word_after_the_icon() {
    assert_eq!(lang_of("\u{e7a8} rust"), "rust");
    assert_eq!(lang_of("rust"), "rust");
    assert_eq!(lang_of("\u{f121} code"), "code");
    assert_eq!(lang_of("RUST,ignore"), "RUST,ignore");
    assert_eq!(lang_of("\u{e7a8}"), "");
}

/// Different families, different blocks — and every block clears the mark
/// floor against the field on every preset, so a badge is never a tint.
#[test]
fn languages_take_distinct_hues_that_stand_off_the_field() {
    let _g = crate::app::theme_test_guard();
    let home = crew_theme::current_id();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let field = crate::chatink::code_bg();
        for lang in ["rust", "python", "js", "sh", "code"] {
            let hue = crate::chathue::lang_hue(lang);
            assert!(
                contrast_ratio(hue, field) >= crate::chatink::CODE_ON_FIELD_FLOOR,
                "{id:?} {lang}: {hue:?} on {field:?}"
            );
        }
    }
    crew_theme::set_theme(home);
    assert_ne!(
        crate::chathue::lang_hue("rust"),
        crate::chathue::lang_hue("python")
    );
}

/// A long language string is clipped so the badge — caps and pads
/// included — still fits the field, one row, on a narrow card.
#[test]
fn a_long_label_is_clipped_to_keep_the_badge_on_one_row() {
    let _g = crate::app::theme_test_guard();
    let _off = force(false);
    let out = lines("```averyveryverylonglanguagename\nx\n```", 16);
    assert_eq!(
        out.len(),
        3,
        "{:?}",
        out.iter().map(row_text).collect::<Vec<_>>()
    );
    assert!(
        row_text(&out[0]).chars().count() <= 17,
        "{:?}",
        row_text(&out[0])
    );
    assert!(
        row_text(&out[0]).ends_with("\u{2026} \u{258c} "),
        "{:?}",
        row_text(&out[0])
    );
}
