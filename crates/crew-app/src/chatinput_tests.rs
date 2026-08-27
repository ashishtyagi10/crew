use super::*;

fn agents(names: &[&str]) -> Vec<AgentInfo> {
    names
        .iter()
        .map(|n| AgentInfo {
            name: (*n).into(),
            role: String::new(),
            model: String::new(),
        })
        .collect()
}

fn row_text(cells: &[CellView], row: u16) -> String {
    // Bucket by column with last-write-wins, as the renderer's grid does, and
    // preserve gaps so left/right alignment is visible.
    let mut line: Vec<char> = Vec::new();
    for c in cells.iter().filter(|c| c.row == row) {
        let col = c.col as usize;
        if line.len() <= col {
            line.resize(col + 1, ' ');
        }
        line[col] = c.c;
    }
    line.into_iter().collect::<String>().trim_end().to_string()
}

#[test]
fn tall_pane_gets_a_bordered_card() {
    let cells = composer_cells("hi", None, &agents(&["planner", "coder"]), 80, 10);
    // Top border (row 7): a plain rounded border — the roster strip is gone
    // (the footer's mode line now names who is WORKING; the full roster
    // lives in the @ picker and footer line 1).
    let top = row_text(&cells, 7);
    assert!(top.starts_with('\u{256d}'), "top: {top}"); // ╭
    assert!(top.ends_with('\u{256e}'), "top: {top}"); // ╮
    assert!(
        !top.contains('@'),
        "roster chips must not ride the border: {top}"
    );
    assert!(
        top.chars()
            .skip(1)
            .take(top.chars().count() - 2)
            .all(|c| c == '\u{2500}'),
        "border must be solid without the chips: {top}"
    );
    // Interior (row 8): side borders around the prompt.
    let mid = row_text(&cells, 8);
    assert!(mid.starts_with("\u{2502} \u{276f} hi"), "mid: {mid}"); // │ ❯ hi
    assert!(mid.ends_with('\u{2502}'), "mid: {mid}"); // │
                                                      // Bottom border (row 9): a plain fieldset edge, no key hints.
    let bot = row_text(&cells, 9);
    assert!(bot.starts_with('\u{2570}'), "bot: {bot}"); // ╰
    assert!(bot.ends_with('\u{256f}'), "bot: {bot}"); // ╯
    assert!(!bot.contains("Esc close"), "bot: {bot}");
}

#[test]
fn short_pane_gets_prompt_only() {
    let cells = composer_cells("hi", None, &agents(&["planner"]), 60, 5);
    assert!(cells.iter().all(|c| c.row == 4));
    assert!(row_text(&cells, 4).starts_with("\u{276f} hi"));
}

#[test]
fn valid_mention_is_highlighted_in_agent_colour() {
    let _g = crate::app::theme_test_guard();
    let a = agents(&["coder"]);
    let cells = composer_cells("@coder fix", None, &a, 60, 10);
    let ink = crew_theme::theme().ink;
    let at = |col: u16| cells.iter().find(|c| c.row == 8 && c.col == col).unwrap();
    // Card interior: `│ ❯ @coder fix` — the mention starts at col 4.
    assert_ne!(at(4).fg, ink, "@ of the mention takes the agent colour");
    assert!(at(4).bold && at(9).bold, "mention renders bold");
    assert_eq!(at(11).fg, ink, "text after the mention stays ink");
}

#[test]
fn unknown_mention_stays_plain() {
    let _g = crate::app::theme_test_guard();
    let cells = composer_cells("@ghost hi", None, &agents(&["coder"]), 60, 10);
    let ink = crew_theme::theme().ink;
    assert!(cells
        .iter()
        .filter(|c| c.row == 8 && c.col >= 4 && c.c != '\u{258f}' && c.c != '\u{2502}')
        .all(|c| c.fg == ink));
}

#[test]
fn caret_follows_the_input() {
    let cells = composer_cells("ab", None, &[], 60, 10);
    let caret = cells.iter().find(|c| c.c == '\u{258f}').unwrap();
    assert_eq!((caret.col, caret.row), (6, 8));
}

#[test]
fn empty_input_shows_a_dim_placeholder_hint() {
    let _g = crate::app::theme_test_guard();
    let cells = composer_cells("", None, &agents(&["planner"]), 60, 10);
    let muted = crew_theme::theme().text_muted;
    // Row 8 is the interior prompt row for a 10-row (tall) pane.
    let hint: String = cells
        .iter()
        .filter(|c| c.row == 8 && c.fg == muted)
        .map(|c| c.c)
        .collect();
    assert!(hint.contains("type a task"), "{hint}");
}

#[test]
fn nonempty_input_has_no_placeholder() {
    let _g = crate::app::theme_test_guard();
    let cells = composer_cells("hi", None, &agents(&["planner"]), 60, 10);
    let muted = crew_theme::theme().text_muted;
    assert!(
        cells.iter().all(|c| c.row != 8 || c.fg != muted),
        "typed input must not render any muted placeholder cells"
    );
    assert!(row_text(&cells, 8).contains("hi"));
}

#[test]
fn placeholder_truncates_to_a_narrow_pane() {
    let cells = composer_cells("", None, &[], 10, 10);
    assert!(cells.iter().all(|c| c.col < 10));
}

#[test]
fn char_count_badge_thresholds() {
    assert_eq!(char_count_badge(10), None);
    assert_eq!(char_count_badge(121), Some("121c".to_string()));
}

#[test]
fn long_input_shows_a_muted_char_count_badge_on_the_top_border() {
    let _g = crate::app::theme_test_guard();
    // 121 chars wrap to 2 interior lines at 75 text columns, so the card's
    // top border sits at rows - 4.
    let long = "a".repeat(121);
    let cells = composer_cells(&long, None, &agents(&["planner"]), 80, 10);
    let muted = crew_theme::theme().text_muted;
    let border_row = 10 - composer_rows(&long, 80, 10);
    let top = row_text(&cells, border_row);
    assert!(top.contains("121c"), "{top}");
    assert!(
        cells
            .iter()
            .any(|c| c.row == border_row && c.c == '1' && c.fg == muted),
        "badge must render in the muted colour"
    );
}

#[test]
fn short_input_has_no_char_count_badge() {
    let _g = crate::app::theme_test_guard();
    let cells = composer_cells("hi", None, &agents(&["planner"]), 80, 10);
    let muted = crew_theme::theme().text_muted;
    assert!(
        cells.iter().all(|c| c.row != 7 || c.fg != muted),
        "no badge expected for short input"
    );
}

#[test]
fn composer_stays_three_rows_for_short_input() {
    assert_eq!(composer_rows("", 80, 12), 3);
    assert_eq!(composer_rows("hi", 80, 12), 3);
}

#[test]
fn composer_grows_when_input_wraps() {
    // cols=20 → 15 text columns per interior line (x0=2, prompt 2, border 1);
    // 40 chars wrap to 3 lines → 3 interior rows + 2 borders.
    let input = "a".repeat(40);
    assert_eq!(composer_rows(&input, 20, 12), 5);
    let cells = composer_cells(&input, None, &[], 20, 12);
    // Interior rows 8, 9, 10 all carry input text; borders at 7 and 11.
    assert!(row_text(&cells, 7).starts_with('\u{256d}'), "top border");
    for row in 8..=10 {
        assert!(row_text(&cells, row).contains('a'), "text on row {row}");
    }
    assert!(
        row_text(&cells, 11).starts_with('\u{2570}'),
        "bottom border"
    );
}

#[test]
fn composer_grows_on_embedded_newlines() {
    assert_eq!(composer_rows("a\nb", 80, 12), 4);
    let cells = composer_cells("a\nb", None, &[], 80, 12);
    assert!(row_text(&cells, 9).contains("\u{276f} a"), "first line");
    assert!(row_text(&cells, 10).contains('b'), "second line");
    // The caret lands after the LAST line, not the first.
    let caret = cells.iter().find(|c| c.c == '\u{258f}').unwrap();
    assert_eq!(caret.row, 10);
}

#[test]
fn composer_growth_is_capped_and_shows_the_tail() {
    // rows=12 caps the interior at 12/3 = 4 lines however long the input.
    let input = (0..20)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(composer_rows(&input, 80, 12), 6);
    let cells = composer_cells(&input, None, &[], 80, 12);
    // Over the cap the view follows the caret: last line ("19") visible,
    // first line ("0") scrolled out.
    assert!(row_text(&cells, 10).contains("19"), "tail line visible");
    assert!(
        !row_text(&cells, 7).contains("\u{276f} 0"),
        "head scrolled out"
    );
}

#[test]
fn short_pane_stays_single_row_even_for_multiline_input() {
    assert_eq!(composer_rows("a\nb\nc", 60, 5), 1);
    let cells = composer_cells("a\nb\nc", None, &[], 60, 5);
    assert!(cells.iter().all(|c| c.row == 4));
}

#[test]
fn input_reduce_accepts_newline_chars() {
    let mut input = String::from("a");
    assert_eq!(input_reduce(&mut input, Some('\n'), false, false), None);
    assert_eq!(input, "a\n");
    // Other control chars are still rejected.
    assert_eq!(input_reduce(&mut input, Some('\u{7}'), false, false), None);
    assert_eq!(input, "a\n");
}

#[test]
fn everything_clips_to_width() {
    let cells = composer_cells(
        "a very long input line that overflows",
        None,
        &agents(&["planner", "coder", "reviewer"]),
        12,
        10,
    );
    assert!(cells.iter().all(|c| c.col < 12));
}

/// The suggestion is drawn dim, after the caret, and never past the row —
/// wrapping it would grow the card under text the user never typed.
#[test]
fn the_ghost_renders_muted_after_the_caret_and_stays_on_the_row() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    let plain = composer_cells("re", None, &[], 40, 3);
    let ghosted = composer_cells("re", Some("factor the parser"), &[], 40, 3);
    assert!(
        ghosted.len() > plain.len(),
        "the suggestion drew nothing at all"
    );
    let extra: Vec<&CellView> = ghosted
        .iter()
        .filter(|c| !plain.iter().any(|p| p.col == c.col && p.row == c.row))
        .collect();
    assert!(
        extra.iter().all(|c| c.fg == t.text_muted),
        "the suggestion must not read as typed text"
    );
    assert!(
        extra.iter().all(|c| c.col < 40),
        "the suggestion ran past the pane"
    );
    let rows: std::collections::BTreeSet<u16> = extra.iter().map(|c| c.row).collect();
    assert_eq!(rows.len(), 1, "the suggestion wrapped onto another row");
    // The card is the same height with and without it.
    let h = |cells: &[CellView]| cells.iter().map(|c| c.row).min().unwrap();
    assert_eq!(h(&plain), h(&ghosted), "the composer grew under the ghost");
}

/// A suggestion longer than the row is clipped, not wrapped — and clipping
/// must not push cells outside the pane or panic on the arithmetic.
#[test]
fn a_ghost_wider_than_the_pane_is_clipped_in_bounds() {
    let long = "x".repeat(500);
    let cells = composer_cells("hi", Some(&long), &[], 24, 4);
    assert!(cells.iter().all(|c| c.col < 24 && c.row < 4));
}
