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
        done_ms: None,
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
    let _g = crate::app::theme_test_guard();
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
    // At 40 cols the title takes three rows; "second" starts right below.
    assert!(rows[3].contains("[ ] second"), "{rows:?}");
}

#[test]
fn clicks_on_wrapped_rows_map_to_the_owning_item() {
    let p = test_pane(vec![item(1, LONG), item(2, "second")]);
    // A continuation row has no checkbox — anywhere on it selects.
    assert_eq!(click_at(&p, 1, 2, COLS, ROWS), Some(TodoClick::Select(0)));
    // The second item's zones sit below the wrapped block.
    assert_eq!(click_at(&p, 3, 2, COLS, ROWS), Some(TodoClick::Toggle(1)));
    assert_eq!(
        click_at(&p, 3, COLS - 2, COLS, ROWS),
        Some(TodoClick::Delete(1))
    );
}

#[test]
fn a_done_item_does_not_render() {
    let mut it = item(1, "done thing");
    it.done = true;
    let p = test_pane(vec![it]);
    let cells = cells(&p, COLS, ROWS);
    assert!(
        !(0..ROWS).any(|r| row_text(&cells, r).contains("done thing")),
        "a ticked item stays off the list until it is asked for"
    );
}

/// The bug this replaced: an all-done pane rendered the SAME "no todos"
/// screen as a brand-new one, so finished work — and the history holding it
/// — was invisible. With nothing selectable there is no way to press `H`
/// either, so the hint names the command that always works.
#[test]
fn an_all_done_list_says_so_instead_of_no_todos() {
    let mut a = item(1, "shipped it");
    a.done = true;
    let mut b = item(2, "and this");
    b.done = true;
    let p = test_pane(vec![a, b]);
    let cells = cells(&p, COLS, ROWS);
    let all: String = (0..ROWS).map(|r| row_text(&cells, r) + "\n").collect();
    assert!(all.contains("all done · 2 in the history"), "{all}");
    assert!(all.contains("/todo done opens the log"), "{all}");
    assert!(
        !all.contains("no todos"),
        "an all-done pane must not read as a fresh one: {all}"
    );
}

/// The empty history under a filter must not claim the whole store is empty.
#[test]
fn an_empty_filtered_history_names_the_filter() {
    let mut p = test_pane(vec![item(1, "open")]);
    p.done_view = true;
    p.filter = Some("home".into());
    let cells = cells(&p, COLS, ROWS);
    let all: String = (0..ROWS).map(|r| row_text(&cells, r) + "\n").collect();
    assert!(all.contains("nothing done in @home"), "{all}");
}

/// The header's done button: the visible half of `h`. It appears only when
/// something is ticked, names the count, and flips to the way back out.
#[test]
fn the_header_carries_a_done_button() {
    let mut done = item(1, "ticked");
    done.done = true;
    let open = item(2, "open");
    let mut p = test_pane(vec![done, open]);
    let header = |p: &_| row_text(&cells(p, COLS, ROWS), 0);
    assert!(header(&p).contains("[show 1 done]"), "{}", header(&p));
    p.set_show_done(true);
    assert!(header(&p).contains("[hide done]"), "{}", header(&p));
    // No button when there is nothing ticked, and none in the history view
    // (which is done-only already).
    let plain = test_pane(vec![item(3, "open")]);
    assert!(!header(&plain).contains("done]"), "{}", header(&plain));
    p.set_done_view(true);
    assert!(!header(&p).contains("done]"), "{}", header(&p));
}

/// And it is a button, not a label: the click lands on the chip's columns
/// only, and the rest of the header row stays inert.
#[test]
fn clicking_the_done_button_toggles_the_done_items() {
    let mut done = item(1, "ticked");
    done.done = true;
    let mut p = test_pane(vec![done, item(2, "open")]);
    let chip_col = COLS - 4; // inside "[show 1 done]", short of the last column
    assert_eq!(
        click_at(&p, 0, chip_col, COLS, ROWS),
        Some(TodoClick::ShowDone)
    );
    assert_eq!(click_at(&p, 0, 1, COLS, ROWS), None, "header text is inert");
    // The item rows still start below the header.
    assert_eq!(click_at(&p, 1, 10, COLS, ROWS), Some(TodoClick::Select(0)));
    p.set_show_done(true);
    assert_eq!(
        click_at(&p, 0, chip_col, COLS, ROWS),
        Some(TodoClick::ShowDone),
        "and it is the way back"
    );
}

#[test]
fn the_composer_legend_previews_a_recognised_due() {
    let _g = crate::app::theme_test_guard();
    let mut p = test_pane(vec![]);
    p.input = "pay rent tomorrow".into();
    p.cursor = p.input.chars().count();
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
    p.cursor = p.input.chars().count();
    assert_eq!(super::super::composer::height(&p, COLS, ROWS), 4);
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
    p.cursor = p.input.chars().count();
    assert_eq!(super::super::composer::height(&p, COLS, ROWS), 2 + 4);
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
    let _g = crate::app::theme_test_guard();
    let mut p = test_pane(vec![]);
    // Pad so "tomorrow" lands on the second wrapped line.
    p.input = "alpha bravo charlie delta echo golf hotel tomorrow".into();
    p.cursor = p.input.chars().count();
    assert_eq!(super::super::composer::height(&p, COLS, ROWS), 4);
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

/// The color contract, all read from ONE `cells()` snapshot: different
/// tags → different chip colors, same tag → identical color, and the
/// composer's live `@tag` tint agrees with the row chip for that tag.
#[test]
fn project_chips_and_composer_tint_share_per_tag_colors() {
    let _g = crate::app::theme_test_guard();
    let mut a = item(1, "one");
    a.project = Some("crew".into());
    let mut b = item(2, "two");
    b.project = Some("home".into());
    let mut c = item(3, "three");
    c.project = Some("crew".into());
    let mut p = test_pane(vec![a, b, c]);
    p.input = "ship it @crew".into();
    p.cursor = p.input.chars().count();
    let cells = cells(&p, COLS, ROWS);
    // Each single-line item sits on its own row, 0..3; the chip is the
    // right-aligned `@…` run, found by its `@` cell.
    let chip_fg = |row: u16| -> (u8, u8, u8) {
        cells
            .iter()
            .find(|cl| cl.row == row && cl.c == '@')
            .unwrap_or_else(|| panic!("no @ chip on row {row}"))
            .fg
    };
    let (crew1, home, crew2) = (chip_fg(0), chip_fg(1), chip_fg(2));
    assert_eq!(crew1, crew2, "same tag must keep one color");
    assert_ne!(crew1, home, "different tags must differ");
    // The composer's `@crew` (bottom rows) tints in the chip's color.
    let composer_at = cells
        .iter()
        .find(|cl| cl.row >= ROWS - 3 && cl.c == '@')
        .expect("no live @ in the composer");
    assert_eq!(composer_at.fg, crew1, "composer tint must match the chip");
    // And neither site is the flat accent anymore, nor the muted tone.
    assert_ne!(crew1, crate::palette::accent());
}

/// With a filter on, the header's `@tag` leads in the tag's color while
/// the ` · N items` tail stays muted.
#[test]
fn the_filter_header_colors_the_tag_but_not_the_tail() {
    let _g = crate::app::theme_test_guard();
    let mut a = item(1, "one");
    a.project = Some("crew".into());
    let mut b = item(2, "two");
    b.project = Some("crew".into());
    let mut p = test_pane(vec![a, b]);
    p.filter = Some("crew".into());
    let cells = cells(&p, COLS, ROWS);
    assert!(row_text(&cells, 0).contains("@crew \u{b7} 2 items"));
    let at = cells
        .iter()
        .find(|cl| cl.row == 0 && cl.c == '@')
        .expect("no @ in header");
    let dot = cells
        .iter()
        .find(|cl| cl.row == 0 && cl.c == '\u{b7}')
        .expect("no · in header");
    let muted = crew_theme::theme().text_muted;
    assert_eq!(dot.fg, muted, "tail must stay muted");
    assert_ne!(at.fg, muted, "tag must leave the muted tone");
    // The chip on the first item row (row 1, under the header) agrees.
    let chip = cells
        .iter()
        .find(|cl| cl.row == 1 && cl.c == '@')
        .expect("no chip on row 1");
    assert_eq!(at.fg, chip.fg, "header and chip must agree on the color");
}

/// The `▏` beam renders at the CURSOR cell, not the text end.
#[test]
fn the_cursor_beam_tracks_a_mid_string_cursor() {
    let mut p = test_pane(vec![]);
    p.input = "hello".into();
    p.cursor = 3;
    let cells = cells(&p, COLS, ROWS);
    let bar = cells
        .iter()
        .find(|c| c.c == '\u{258f}')
        .expect("no cursor beam");
    // Interior text starts at col 4; three chars in → col 7, over the
    // second 'l' (the beam draws last, compositing over the glyph).
    assert_eq!(bar.col, 7);
    assert!(
        cells
            .iter()
            .any(|c| c.c == 'l' && c.col == 7 && c.row == bar.row),
        "the glyph under the beam still renders"
    );
}

/// The tag popup's rows render in each project's own color — the same
/// triple the row chips use, so the picker previews what you'll get.
#[test]
fn tag_popup_rows_carry_their_project_colors() {
    let _g = crate::app::theme_test_guard();
    let mut a = item(1, "one");
    a.project = Some("crew".into());
    let mut b = item(2, "two");
    b.project = Some("home".into());
    let mut p = test_pane(vec![a, b]);
    p.type_char('@'); // opens the tag popup over the known tags
    assert!(p.tagmenu.is_some(), "premise: popup open");
    let cells = cells(&p, COLS, ROWS);
    let t = crew_theme::theme();
    let popup_fg = |tag: &str| {
        let want: String = format!("@{tag}");
        // Find the popup row whose text contains "@tag" and read the '@' fg.
        let at_cells: Vec<_> = cells.iter().filter(|c| c.c == '@').collect();
        at_cells
            .iter()
            .find(|c| {
                let row_text = row_text(&cells, c.row);
                row_text.contains(&want)
            })
            .unwrap_or_else(|| panic!("no popup row for @{tag}"))
            .fg
    };
    assert_eq!(popup_fg("crew"), crew_theme::tag_color("crew", t));
    assert_eq!(popup_fg("home"), crew_theme::tag_color("home", t));
    assert_ne!(popup_fg("crew"), popup_fg("home"));
}

// --- done history (v0.17: `/todo done`) -----------------------------------

fn done_item(id: u64, title: &str, stamp: Option<u64>) -> TodoItem {
    let mut it = item(id, title);
    it.done = true;
    it.done_ms = stamp;
    it
}

/// Epoch ms for today's (or a nearby day's) local wall clock — the render
/// buckets by LOCAL date, so fixtures must be built the same way.
fn local_ms(days_ago: i64, h: u32, m: u32) -> u64 {
    let d = duedate::now_local().date() - chrono::Duration::days(days_ago);
    duedate::to_epoch_ms(d.and_hms_opt(h, m, 0).unwrap()).unwrap()
}

#[test]
fn the_history_groups_under_day_headers_with_tick_times() {
    let mut p = test_pane(vec![
        done_item(1, "new", Some(local_ms(0, 14, 30))),
        done_item(2, "old", Some(local_ms(1, 9, 15))),
        done_item(3, "ancient", None), // pre-stamp legacy tick
    ]);
    p.done_view = true;
    let cells = cells(&p, COLS, ROWS);
    assert!(
        row_text(&cells, 0).contains("today"),
        "{}",
        row_text(&cells, 0)
    );
    let r1 = row_text(&cells, 1);
    assert!(r1.contains("[x] new") && r1.contains("14:30"), "{r1}");
    assert!(
        row_text(&cells, 2).contains("yesterday"),
        "{}",
        row_text(&cells, 2)
    );
    let r3 = row_text(&cells, 3);
    assert!(r3.contains("[x] old") && r3.contains("09:15"), "{r3}");
    assert!(
        row_text(&cells, 4).contains("earlier"),
        "legacy ticks group under a stampless header: {}",
        row_text(&cells, 4)
    );
    let r5 = row_text(&cells, 5);
    assert!(r5.contains("[x] ancient"), "{r5}");
    assert!(!r5.contains(':'), "no fake time on a stampless row: {r5}");
}

#[test]
fn history_headers_are_not_clickable_rows() {
    let mut p = test_pane(vec![
        done_item(1, "new", Some(local_ms(0, 14, 30))),
        done_item(2, "old", Some(local_ms(1, 9, 15))),
    ]);
    p.done_view = true;
    assert_eq!(
        click_at(&p, 0, 10, COLS, ROWS),
        None,
        "a day header is inert"
    );
    assert_eq!(click_at(&p, 1, 10, COLS, ROWS), Some(TodoClick::Select(0)));
    assert_eq!(click_at(&p, 1, 2, COLS, ROWS), Some(TodoClick::Toggle(0)));
    assert_eq!(click_at(&p, 2, 10, COLS, ROWS), None, "second header inert");
    assert_eq!(click_at(&p, 3, 10, COLS, ROWS), Some(TodoClick::Select(1)));
    assert_eq!(
        click_at(&p, 4, 10, COLS, ROWS),
        None,
        "past the log: nothing"
    );
}

#[test]
fn an_empty_history_says_so_quietly() {
    let mut p = test_pane(vec![item(1, "still open")]);
    p.done_view = true;
    let cells = cells(&p, COLS, ROWS);
    let all: String = (0..ROWS).map(|r| row_text(&cells, r) + "\n").collect();
    assert!(all.contains("nothing done yet"), "{all}");
    assert!(
        !all.contains("still open"),
        "open items never leak into the history: {all}"
    );
}

/// A pane too narrow for the button keeps the row for the list — the button
/// is not drawn there, so it must not reserve space either.
#[test]
fn a_narrow_pane_spends_no_row_on_a_button_it_cannot_draw() {
    let mut done = item(1, "ticked");
    done.done = true;
    let p = test_pane(vec![done, item(2, "open")]);
    let narrow = 18; // "[show 1 done]" needs more than this
    let cells = cells(&p, narrow, ROWS);
    assert!(!row_text(&cells, 0).contains("done]"), "no room for it");
    assert!(
        row_text(&cells, 0).contains("open"),
        "so row 0 is the list: {:?}",
        row_text(&cells, 0)
    );
    assert_eq!(
        click_at(&p, 0, 10, narrow, ROWS),
        Some(TodoClick::Select(0))
    );
}

/// The title and the chip beside it keep the same two columns of air every
/// other pair on the row keeps. At one, a title that happens to fill its
/// budget reads straight into the tag — `…and reverts @crew` was one phrase.
#[test]
fn the_title_keeps_two_columns_before_the_chip_beside_it() {
    let mut it = item(1, &"x".repeat(80)); // fills the first line exactly
    it.project = Some("home".into());
    let p = test_pane(vec![it]);
    let row = row_text(&cells(&p, 60, ROWS), 0);
    let at = row
        .find('@')
        .expect("the tag rides the first row at 60 cols");
    let title_end = row[..at].trim_end().len();
    assert!(
        at - title_end >= 2,
        "one column of air reads as one phrase: {row:?}"
    );
}

/// On a pane too narrow to share the line, the chips drop to a row of their
/// own rather than squeezing the title into a hard break: at 30 cols
/// `ship the release notes` had three columns and came out as `shi` / `p the
/// release notes`.
#[test]
fn a_narrow_row_stacks_its_chips_below_the_title() {
    let _g = crate::app::theme_test_guard();
    let mut it = item(1, "ship the release notes");
    it.project = Some("crew".into());
    let p = test_pane(vec![it]);
    let cells = cells(&p, 30, ROWS);
    let rows: Vec<String> = (0..5).map(|r| row_text(&cells, r)).collect();
    assert!(
        !rows[0].contains('@'),
        "the chips leave the title's line: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|r| r.contains("@crew") && !r.contains("[ ]")),
        "and land on a row of their own: {rows:?}"
    );
    for w in "ship the release notes".split_whitespace() {
        assert!(rows.iter().any(|r| r.contains(w)), "{w} broken: {rows:?}");
    }
}

/// The stacked row's `✗` moves with it, and the checkbox does not.
#[test]
fn a_stacked_row_takes_its_delete_target_down_with_it() {
    let mut it = item(1, "ship the release notes");
    it.project = Some("crew".into());
    let p = test_pane(vec![it]);
    let cols = 30;
    // Two title rows at this width, then the chips.
    assert_eq!(click_at(&p, 0, 2, cols, ROWS), Some(TodoClick::Toggle(0)));
    assert_eq!(
        click_at(&p, 0, cols - 3, cols, ROWS),
        Some(TodoClick::Select(0)),
        "the first row's right end is no longer the ✗"
    );
    let last = item_h(&p.items[0], cols, crate::chattime::unix_now_ms(), false) - 1;
    assert_eq!(
        click_at(&p, last, cols - 3, cols, ROWS),
        Some(TodoClick::Delete(0))
    );
}

/// A row wide enough to share keeps everything on one line — stacking a
/// short title beside a tag and a due would buy nothing and cost a row.
#[test]
fn a_short_title_never_stacks_just_because_the_pane_is_narrowish() {
    let mut it = item(1, "pay rent");
    it.project = Some("home".into());
    it.due_ms = Some(1_000);
    let p = test_pane(vec![it]);
    assert_eq!(
        item_h(&p.items[0], 40, crate::chattime::unix_now_ms(), false),
        1
    );
}
