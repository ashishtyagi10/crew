use super::*;
use crate::todopane::item::TodoItem;
use crate::todopane::test_pane;

const COLS: u16 = 40;
const ROWS: u16 = 20;

fn item(id: u64, title: &str) -> TodoItem {
    TodoItem {
        id,
        title: title.to_string(),
        done: false,
        project: None,
        due_ms: None,
        due_has_time: false,
        created_ms: id,
        notified: false,
    }
}

/// The text content of one row, cells sorted by column, gaps as spaces.
fn row_text(cells: &[crew_render::CellView], row: u16) -> String {
    let mut on_row: Vec<&crew_render::CellView> = cells.iter().filter(|c| c.row == row).collect();
    on_row.sort_by_key(|c| c.col);
    let mut s = String::new();
    let mut x = 0;
    for c in on_row {
        while x < c.col {
            s.push(' ');
            x += 1;
        }
        s.push(c.c);
        x += crate::chatwidth::char_w(c.c) as u16;
    }
    s
}

#[test]
fn clicks_map_to_checkbox_delete_row_and_composer() {
    let p = test_pane(vec![item(1, "one"), item(2, "two")]);
    // Row 0, checkbox cells [BOX_COL..BOX_COL+3).
    assert_eq!(click_at(&p, 0, 2, COLS, ROWS), Some(TodoClick::Toggle(0)));
    assert_eq!(click_at(&p, 0, 4, COLS, ROWS), Some(TodoClick::Toggle(0)));
    // The ✗ zone at the row's end.
    assert_eq!(
        click_at(&p, 1, COLS - 2, COLS, ROWS),
        Some(TodoClick::Delete(1))
    );
    // Anywhere else on a row selects it.
    assert_eq!(click_at(&p, 1, 10, COLS, ROWS), Some(TodoClick::Select(1)));
    // Below the last item: nothing (falls through to the app's focus path).
    assert_eq!(click_at(&p, 2, 10, COLS, ROWS), None);
    // The composer zone (bottom 3 rows) refocuses the composer.
    assert_eq!(
        click_at(&p, ROWS - 1, 5, COLS, ROWS),
        Some(TodoClick::Composer)
    );
    assert_eq!(
        click_at(&p, ROWS - 3, 5, COLS, ROWS),
        Some(TodoClick::Composer)
    );
}

#[test]
fn an_active_filter_adds_a_header_row_that_shifts_the_list() {
    let mut p = test_pane(vec![item(1, "one")]);
    p.filter = Some("crew".into());
    p.items[0].project = Some("crew".into());
    // Row 0 is the info line now — not clickable as an item…
    assert_eq!(click_at(&p, 0, 10, COLS, ROWS), None);
    // …and the first item moved to row 1.
    assert_eq!(click_at(&p, 1, 10, COLS, ROWS), Some(TodoClick::Select(0)));
    let cells = cells(&p, COLS, ROWS);
    assert!(
        row_text(&cells, 0).contains("@crew · 1 item"),
        "the header names the filter: {:?}",
        row_text(&cells, 0)
    );
}

#[test]
fn scroll_offsets_the_click_mapping() {
    let mut p = test_pane((0..30).map(|i| item(i, &format!("t{i}"))).collect());
    p.scroll = 5;
    assert_eq!(click_at(&p, 0, 10, COLS, ROWS), Some(TodoClick::Select(5)));
}

#[test]
fn rows_render_checkbox_title_tag_due_and_delete_affordance() {
    let mut it = item(1, "pay rent");
    it.project = Some("home".into());
    // Overdue by construction: any past stamp.
    it.due_ms = Some(1_000);
    let p = test_pane(vec![it]);
    let cells = cells(&p, COLS, ROWS);
    let row = row_text(&cells, 0);
    assert!(row.contains("[ ] pay rent"), "{row:?}");
    assert!(row.contains("@home"), "{row:?}");
    assert!(row.contains('\u{2717}'), "the delete ✗ is present: {row:?}");
    // The overdue label renders in the bell color.
    let bell = crew_theme::theme().bell;
    assert!(
        cells.iter().any(|c| c.row == 0 && c.fg == bell),
        "an overdue due label uses the bell color"
    );
}

const LONG: &str = "alpha bravo charlie delta echo foxtrot golf hotel india juliet";

#[test]
fn a_long_title_wraps_and_shows_every_word() {
    let p = test_pane(vec![item(1, LONG), item(2, "second")]);
    let cells = cells(&p, COLS, ROWS);
    let rows: Vec<String> = (0..5).map(|r| row_text(&cells, r)).collect();
    for w in LONG.split_whitespace() {
        assert!(rows.iter().any(|r| r.contains(w)), "{w} missing: {rows:?}");
    }
    assert!(rows[0].contains("[ ] alpha"), "{rows:?}");
    // At 40 cols the title takes two rows; "second" starts right below.
    assert!(rows[2].contains("[ ] second"), "{rows:?}");
}

#[test]
fn clicks_on_wrapped_rows_map_to_the_owning_item() {
    let p = test_pane(vec![item(1, LONG), item(2, "second")]);
    // A continuation row has no checkbox — anywhere on it selects.
    assert_eq!(click_at(&p, 1, 2, COLS, ROWS), Some(TodoClick::Select(0)));
    // The second item's zones sit below the wrapped block.
    assert_eq!(click_at(&p, 2, 2, COLS, ROWS), Some(TodoClick::Toggle(1)));
    assert_eq!(
        click_at(&p, 2, COLS - 2, COLS, ROWS),
        Some(TodoClick::Delete(1))
    );
}

#[test]
fn a_done_item_does_not_render() {
    let mut it = item(1, "done thing");
    it.done = true;
    let p = test_pane(vec![it]);
    let cells = cells(&p, COLS, ROWS);
    let row = row_text(&cells, 0);
    assert!(!row.contains("done thing"), "{row:?}");
    let all: String = (0..ROWS).map(|r| row_text(&cells, r)).collect();
    assert!(
        all.contains("no todos"),
        "an all-done list shows the empty hint"
    );
}

#[test]
fn the_composer_legend_previews_a_recognised_due() {
    let mut p = test_pane(vec![]);
    p.input = "pay rent tomorrow".into();
    let cells = cells(&p, COLS, ROWS);
    let border = row_text(&cells, ROWS - 3);
    assert!(border.contains("due tomorrow"), "{border:?}");
    // And the fragment itself is tinted accent in the prompt row.
    let accent = crate::palette::accent();
    let prompt = ROWS - 2;
    let tinted: String = {
        let mut v: Vec<&crew_render::CellView> = cells
            .iter()
            .filter(|c| c.row == prompt && c.fg == accent && c.c != '\u{276f}' && c.c != '\u{258f}')
            .collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert_eq!(tinted, "tomorrow");
}

#[test]
fn an_empty_pane_hints_at_the_syntax() {
    let p = test_pane(vec![]);
    let all = cells(&p, COLS, ROWS);
    let text: String = (0..ROWS).map(|r| row_text(&all, r) + "\n").collect();
    assert!(text.contains("no todos"), "{text}");
    assert!(text.contains("type one below"), "{text}");
}

#[test]
fn list_height_accounts_for_composer_popup_and_header() {
    let mut p = test_pane(vec![item(1, "a")]);
    assert_eq!(list_height(&p, COLS, ROWS), ROWS - 3);
    p.filter = Some("crew".into());
    assert_eq!(list_height(&p, COLS, ROWS), ROWS - 4);
    p.tagmenu = Some(crate::todopane::tagmenu::TagMenu {
        matches: vec!["crew".into()],
        sel: 0,
    });
    // One match → 1 row + 2 border rows of popup.
    assert_eq!(list_height(&p, COLS, ROWS), ROWS - 4 - 3);
}

#[test]
fn a_long_composer_input_wraps_onto_a_second_row() {
    let mut p = test_pane(vec![item(1, "x")]);
    // 49 cells > the 34-cell interior budget at 40 cols → two lines.
    p.input = "alpha bravo charlie delta echo foxtrot golf hotel".into();
    assert_eq!(composer_h(&p, COLS, ROWS), 4);
    assert_eq!(list_height(&p, COLS, ROWS), ROWS - 4);
    let cells = cells(&p, COLS, ROWS);
    let first = row_text(&cells, ROWS - 3);
    let second = row_text(&cells, ROWS - 2);
    assert!(first.contains("alpha"), "{first:?}");
    assert!(!first.contains("hotel"), "{first:?}");
    assert!(second.contains("foxtrot golf hotel"), "{second:?}");
    // The cursor sits at the end of the wrapped tail, not the first row.
    assert!(second.contains('\u{258f}'), "{second:?}");
    assert!(!first.contains('\u{258f}'), "{first:?}");
    // The grown card is all composer to clicks.
    assert_eq!(
        click_at(&p, ROWS - 4, 5, COLS, ROWS),
        Some(TodoClick::Composer)
    );
    // And the item list ends where the card now begins.
    assert_eq!(click_at(&p, ROWS - 5, 10, COLS, ROWS), None);
}

#[test]
fn composer_growth_caps_and_keeps_the_tail_visible() {
    let mut p = test_pane(vec![]);
    // 40 words wrap to 6 lines at 40 cols — past the 4-line cap.
    p.input = "word ".repeat(39) + "last";
    assert_eq!(composer_h(&p, COLS, ROWS), 2 + 4);
    let cells = cells(&p, COLS, ROWS);
    let bottom = row_text(&cells, ROWS - 2);
    assert!(bottom.contains("last"), "{bottom:?}");
    assert!(bottom.contains('\u{258f}'), "{bottom:?}");
    // The first interior row is mid-text (the head scrolled away).
    let top_row = row_text(&cells, ROWS - 5);
    assert!(top_row.contains("word"), "{top_row:?}");
}

#[test]
fn a_date_fragment_stays_tinted_on_a_wrapped_row() {
    let mut p = test_pane(vec![]);
    // Pad so "tomorrow" lands on the second wrapped line.
    p.input = "alpha bravo charlie delta echo golf hotel tomorrow".into();
    assert_eq!(composer_h(&p, COLS, ROWS), 4);
    let cells = cells(&p, COLS, ROWS);
    let accent = crate::palette::accent();
    let tinted: String = {
        let mut v: Vec<&crew_render::CellView> = cells
            .iter()
            .filter(|c| {
                c.row == ROWS - 2 && c.fg == accent && c.c != '\u{276f}' && c.c != '\u{258f}'
            })
            .collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert_eq!(tinted, "tomorrow");
}
