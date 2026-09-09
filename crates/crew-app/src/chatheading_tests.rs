use crate::chatbody::{CardLine, Color};

fn lines(text: &str, width: usize, fg: Color) -> Vec<CardLine> {
    crate::chatmd::map_lines(crate::md::render_chat(text, width), width, fg)
}

fn row_text(line: &CardLine) -> String {
    line.iter().map(|c| c.c).collect()
}

/// On main `# Title` was ONE row: `out.len()` was 1 and no rule followed.
#[test]
fn an_h1_is_ruled_under_as_wide_as_its_text() {
    let _guard = crate::app::theme_test_guard();
    let out = lines("# Title", 40, (9, 9, 9));
    assert_eq!(
        out.len(),
        2,
        "{:?}",
        out.iter().map(row_text).collect::<Vec<_>>()
    );
    assert_eq!(
        row_text(&out[1]),
        " \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}"
    );
    assert!(out[1][1..]
        .iter()
        .all(|c| c.fg == crew_theme::theme().text_muted && !c.bold));
}

/// Lower levels are not ruled, and a wide glyph counts two columns.
#[test]
fn only_h1_gets_the_rule_and_the_rule_counts_display_columns() {
    let _guard = crate::app::theme_test_guard();
    assert_eq!(lines("## Two", 40, (9, 9, 9)).len(), 1);
    assert_eq!(lines("### Three", 40, (9, 9, 9)).len(), 1);
    let wide = lines("# 日本", 40, (9, 9, 9));
    assert_eq!(row_text(&wide[1]), format!(" {}", "\u{2500}".repeat(4)));
}

/// A heading that wraps is ruled once, under its last row.
#[test]
fn a_wrapped_h1_is_ruled_once_under_its_last_row() {
    let _guard = crate::app::theme_test_guard();
    let out = lines("# one two three", 8, (9, 9, 9));
    let rows: Vec<String> = out.iter().map(row_text).collect();
    assert_eq!(
        rows,
        vec![
            " one two",
            " three",
            " \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}"
        ]
    );
}

/// The VIEWER does not rule its headings — that is the card's reading.
#[test]
fn the_viewer_path_leaves_h1_unruled() {
    let _guard = crate::app::theme_test_guard();
    let (rows, _) = crate::chatmd::with_pictures(crate::md::render("# T", 40), 40, (9, 9, 9));
    assert_eq!(rows.len(), 1);
}
