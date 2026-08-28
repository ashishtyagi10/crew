use super::*;

#[test]
fn a_pane_with_nothing_new_has_no_divider() {
    assert_eq!(divider_row(100, 100, 20, 0), None);
    assert_eq!(divider_row(90, 100, 20, 0), None, "a cleared buffer");
    assert_eq!(count(100, 100), 0);
    assert_eq!(count(90, 100), 0, "clearing is not negative news");
    assert_eq!(count(112, 100), 12);
}

/// The rule sits under the last line that was read: with 100 lines read and
/// 120 in the buffer, a 20-row window shows lines 100..120 — the boundary is
/// one line above the top, so there is nothing to draw.
#[test]
fn a_boundary_above_the_window_is_not_drawn() {
    assert_eq!(divider_row(120, 100, 20, 0), None);
}

/// Scrolled back so the boundary is on screen, it lands on the row holding
/// the last-read line.
#[test]
fn the_rule_lands_on_the_last_line_that_was_read() {
    // 120 lines, window of 20, scrolled back 10 → showing lines 90..110.
    // Line 99 (the 100th) is the last read one, on row 9.
    assert_eq!(divider_row(120, 100, 20, 10), Some(9));
    // Scrolled back further: the same buffer line moves down the window.
    assert_eq!(divider_row(120, 100, 20, 15), Some(14));
}

/// A rule on the bottom row divides nothing — everything new is off screen
/// below it, so the mark would be a line under the whole window.
#[test]
fn a_boundary_on_the_last_visible_row_is_not_drawn() {
    // Showing lines 81..101: line 99 is row 18 of 20 — fine.
    assert_eq!(divider_row(120, 100, 20, 19), Some(18));
    // One further back puts it on row 19, the last: nothing follows it.
    assert_eq!(divider_row(120, 100, 20, 20), None);
}

/// A pane that has never been read (`read_at` 0) has no boundary to draw:
/// everything is new, which is what an empty pane looks like anyway.
#[test]
fn a_pane_read_at_zero_draws_nothing() {
    assert_eq!(divider_row(50, 0, 20, 0), None);
}

/// Every cell on the row is ruled, including the blanks between words — a
/// divider with gaps in it is a dashed line nobody drew on purpose.
#[test]
fn the_rule_spans_the_whole_row_including_its_gaps() {
    let _g = crate::app::theme_test_guard();
    let mut cells = vec![
        CellView {
            col: 0,
            row: 3,
            c: 'a',
            ..Default::default()
        },
        CellView {
            col: 5,
            row: 3,
            c: 'b',
            ..Default::default()
        },
        CellView {
            col: 2,
            row: 4,
            c: 'c',
            ..Default::default()
        },
    ];
    mark(&mut cells, 3, 8, 4);
    let ruled: Vec<u16> = cells
        .iter()
        .filter(|c| c.row == 3 && c.deco.line == DecoLine::Single)
        .map(|c| c.col)
        .collect();
    let mut sorted = ruled.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, (0..8).collect::<Vec<_>>(), "{ruled:?}");
    assert_eq!(
        cells.iter().find(|c| c.row == 4).unwrap().deco.line,
        DecoLine::None,
        "the rule reached another row"
    );
    // The glyphs are untouched — the rule is drawn under them, not over.
    assert_eq!(
        cells.iter().find(|c| c.col == 0 && c.row == 3).unwrap().c,
        'a'
    );
}

#[test]
fn the_badge_caps_rather_than_growing_a_fourth_digit() {
    assert_eq!(badge(0), None);
    assert_eq!(badge(1).as_deref(), Some("1"));
    assert_eq!(badge(99).as_deref(), Some("99"));
    assert_eq!(badge(100).as_deref(), Some("99+"));
    assert_eq!(badge(4000).as_deref(), Some("99+"));
}

/// On the card: the count rides the top border beside the activity dot, and a
/// pane with nothing new draws no number at all.
#[test]
fn the_card_shows_the_count_and_only_when_there_is_one() {
    let _g = crate::app::theme_test_guard();
    let bar = |unread| crate::panecard::Bar {
        index: Some(2),
        title: "sh",
        focused: false,
        scroll: 0,
        total: 0,
        activity: true,
        bell: false,
        broadcast: false,
        min_btn: false,
        assemble_t: 1.0,
        focus_t: 1.0,
        git: None,
        ticks: &[],
        hits: &[],
        progress: None,
        elapsed: None,
        pinned: false,
        at_cmd: None,
        fail_rows: &[],
        cmd_rows: &[],
        err_rows: &[],
        unread,
        doc: false,
    };
    let row0 = |unread| -> String {
        let mut cells = crate::panecard::pane_card(60, 8, &bar(unread));
        cells.retain(|c| c.row == 0);
        cells.sort_by_key(|c| c.col);
        cells.iter().map(|c| c.c).collect()
    };
    assert!(row0(12).contains("12"), "{}", row0(12));
    assert!(!row0(0).contains("12"));
    assert!(
        row0(0).contains('\u{25cf}'),
        "the activity dot went missing"
    );
}

/// The rule in the pane you are looking at is the bug this fix exists for: a
/// full-width line stranded mid-output, in the theme's `activity` colour,
/// with nothing on screen saying what it is.
#[test]
fn the_pane_you_are_reading_never_strands_a_rule() {
    // 100 lines read, 12 arrived while you watched, a 20-row window at the
    // live bottom.
    let stale = 100;
    let total = 112;
    // What the old guard produced: it advanced the mark only when
    // `count == 0`, which is only true when the mark is already at the tail —
    // so it never fired, and the rule sat eight rows up the window.
    assert_eq!(count(total, stale), 12, "the guard's own condition");
    assert_eq!(divider_row(total, stale, 20, 0), Some(7));
    // Focused, at the live bottom: the mark follows the tail and there is no
    // boundary left to draw.
    let read = follow_tail(stale, true, true, total);
    assert_eq!(read, total);
    assert_eq!(divider_row(total, read, 20, 0), None);
}

/// The two cases the divider is *for* keep their boundary.
#[test]
fn a_pane_you_are_not_reading_keeps_its_boundary() {
    // Not focused: this is the case the module exists for.
    assert_eq!(follow_tail(100, false, true, 112), 100);
    // Focused but scrolled back: you are catching up, and the rule is what
    // you are catching up to.
    assert_eq!(follow_tail(100, true, false, 112), 100);
    // Showing lines 82..102 of 112: the boundary at line 99 is row 17.
    assert_eq!(
        divider_row(112, follow_tail(100, true, false, 112), 20, 10),
        Some(17)
    );
}

/// An anonymous full-width line reads as damage. The rule says what it is,
/// right-aligned in the row's trailing blanks.
#[test]
fn the_rule_names_what_it_divides() {
    let _g = crate::app::theme_test_guard();
    let mut cells = vec![CellView {
        col: 0,
        row: 1,
        c: 'x',
        ..Default::default()
    }];
    mark(&mut cells, 1, 20, 12);
    let mut row: Vec<&CellView> = cells.iter().filter(|c| c.row == 1).collect();
    row.sort_by_key(|c| c.col);
    let text: String = row.iter().map(|c| c.c).collect();
    assert_eq!(text, "x             12 new", "{text:?}");
    // The tag wears the rule's own colour, and the rule still spans the row.
    let fg = crew_theme::theme().activity;
    assert!(row.iter().all(|c| c.deco.line == DecoLine::Single));
    assert!(row
        .iter()
        .filter(|c| c.c != ' ' && c.col > 0)
        .all(|c| c.fg == fg));
}

/// A row whose own output reaches the right edge keeps every glyph: covering
/// a line of output to explain a rule is the worse trade.
#[test]
fn a_full_row_keeps_its_output_and_goes_bare() {
    let _g = crate::app::theme_test_guard();
    let mut cells: Vec<CellView> = (0..10u16)
        .map(|col| CellView {
            col,
            row: 0,
            c: 'x',
            ..Default::default()
        })
        .collect();
    mark(&mut cells, 0, 10, 12);
    let text: String = {
        let mut r: Vec<&CellView> = cells.iter().filter(|c| c.row == 0).collect();
        r.sort_by_key(|c| c.col);
        r.iter().map(|c| c.c).collect()
    };
    assert_eq!(text, "xxxxxxxxxx");
    // ...and one blank column is not enough for a tag either.
    let mut narrow: Vec<CellView> = (0..3u16)
        .map(|col| CellView {
            col,
            row: 0,
            c: 'x',
            ..Default::default()
        })
        .collect();
    mark(&mut narrow, 0, 10, 12);
    let text: String = {
        let mut r: Vec<&CellView> = narrow.iter().filter(|c| c.row == 0).collect();
        r.sort_by_key(|c| c.col);
        r.iter().map(|c| c.c).collect()
    };
    assert_eq!(text, "xxx       ", "the tag crowded the output");
    // ...one column further left and it fits, with its two blanks intact.
    let mut room: Vec<CellView> = (0..2u16)
        .map(|col| CellView {
            col,
            row: 0,
            c: 'x',
            ..Default::default()
        })
        .collect();
    mark(&mut room, 0, 10, 12);
    let text: String = {
        let mut r: Vec<&CellView> = room.iter().filter(|c| c.row == 0).collect();
        r.sort_by_key(|c| c.col);
        r.iter().map(|c| c.c).collect()
    };
    assert_eq!(text, "xx  12 new");
}

/// A pane with nothing new draws no rule at all, so the tag has no zero case
/// to render — but the count it prints is the card's, capped the same way.
#[test]
fn the_tag_caps_the_way_the_badge_does() {
    assert_eq!(tag(12, 20, None), Some((14, "12 new".to_string())));
    assert_eq!(tag(4000, 20, None), Some((13, "99+ new".to_string())));
    assert_eq!(tag(0, 20, None), None);
    assert_eq!(tag(12, 5, None), None, "no room for the tag");
}
