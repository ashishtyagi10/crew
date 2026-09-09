use crate::chatbody::{CardLine, Color};

fn lines(text: &str, width: usize, fg: Color) -> Vec<CardLine> {
    crate::chatmd::map_lines(crate::md::render_chat(text, width), width, fg)
}

fn row_text(line: &CardLine) -> String {
    line.iter().map(|c| c.c).collect()
}

/// On main `# Title` was ONE row: `out.len()` was 1 and no rule followed.
/// The rule runs under the badge and the title alike: it is the row's.
#[test]
fn an_h1_is_ruled_under_as_wide_as_its_row() {
    let _guard = crate::app::theme_test_guard();
    let _off = crate::glyphs::force(false);
    let out = lines("# Title", 40, (9, 9, 9));
    assert_eq!(
        out.len(),
        2,
        "{:?}",
        out.iter().map(row_text).collect::<Vec<_>>()
    );
    assert_eq!(row_text(&out[0]), " \u{2590} # \u{258c} Title");
    assert_eq!(row_text(&out[1]), format!(" {}", "\u{2500}".repeat(11)));
    assert!(out[1][1..]
        .iter()
        .all(|c| c.fg == crew_theme::theme().text_muted && !c.bold));
}

/// Lower levels are not ruled, and a wide glyph counts two columns.
#[test]
fn only_h1_gets_the_rule_and_the_rule_counts_display_columns() {
    let _guard = crate::app::theme_test_guard();
    let _off = crate::glyphs::force(false);
    assert_eq!(lines("## Two", 40, (9, 9, 9)).len(), 1);
    assert_eq!(lines("### Three", 40, (9, 9, 9)).len(), 1);
    let wide = lines("# 日本", 40, (9, 9, 9));
    assert_eq!(row_text(&wide[1]), format!(" {}", "\u{2500}".repeat(10)));
}

/// A heading that wraps is ruled once, under its last row. The badge leads
/// the FIRST row only, and the layout wraps an h1 `BADGE_W` narrower so the
/// badge never pushes the title's last word onto a row of its own.
#[test]
fn a_wrapped_h1_is_ruled_once_under_its_last_row() {
    let _guard = crate::app::theme_test_guard();
    let _off = crate::glyphs::force(false);
    let out = lines("# one two three", 8 + super::BADGE_W, (9, 9, 9));
    let rows: Vec<String> = out.iter().map(row_text).collect();
    assert_eq!(
        rows,
        vec![
            " \u{2590} # \u{258c} one two",
            " three",
            " \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}"
        ]
    );
    assert_eq!(super::wrap_cols(1, 14), 8);
    assert_eq!(super::wrap_cols(2, 14), 14);
}

/// The badge is the accent as a block, its mark in ink that clears the text
/// floor on it; the title beside it keeps the accent as ink.
#[test]
fn the_h1_badge_is_the_accent_block_with_a_readable_mark() {
    let _guard = crate::app::theme_test_guard();
    let _off = crate::glyphs::force(false);
    let out = lines("# Title", 40, (9, 9, 9));
    let accent = crate::palette::accent();
    assert_eq!((out[0][1].fg, out[0][1].bg), (accent, None), "cap");
    assert_eq!(out[0][3].c, '#');
    assert_eq!(out[0][3].bg, Some(accent));
    assert!(crew_theme::contrast_ratio(out[0][3].fg, accent) >= crew_theme::contrast::text_floor());
    assert_eq!(out[0][7].fg, accent, "the title's ink");
    assert!(out[0][6].bg.is_none(), "the gap sits on the page");
    let _on = crate::glyphs::force(true);
    let on = lines("# T", 40, (9, 9, 9));
    assert_eq!(on[0][3].c, '\u{f292}', "the hashtag icon on a Nerd Font");
}

/// The VIEWER does not rule its headings — that is the card's reading.
#[test]
fn the_viewer_path_leaves_h1_unruled() {
    let _guard = crate::app::theme_test_guard();
    let (rows, _) = crate::chatmd::with_pictures(crate::md::render("# T", 40), 40, (9, 9, 9));
    assert_eq!(rows.len(), 1);
}
