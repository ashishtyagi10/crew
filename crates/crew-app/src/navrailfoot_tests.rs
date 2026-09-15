use super::*;

fn git_at(changed: usize, ahead: usize, behind: usize) -> GitInfo {
    GitInfo {
        branch: "main".into(),
        changed,
        ahead,
        behind,
    }
}

/// Rows the plan actually claims, the gaps included — what the draw loop
/// walks, derived the same way the draw loop derives it.
fn span(top: u16, shown: &[Group], f: &Foot) -> (u16, u16) {
    (top, top + height(shown, f))
}

/// A foot with everything to say: a repo, a sky, a machine under load.
fn full(git: Option<GitInfo>) -> Foot {
    Foot {
        time: "09:41".into(),
        sky: Some("\u{2601}12\u{00b0}".into()),
        stats: Stats {
            cpu: 0.4,
            mem: 0.7,
            disk: 0.5,
            net_rx: 4096,
            net_tx: 12,
        },
        git,
    }
}

#[test]
fn a_full_height_rail_shows_every_reading_the_open_nav_does() {
    let f = full(Some(git_at(9, 1, 0)));
    let (top, shown) = plan(&f, 48, 3);
    assert_eq!(shown, ORDER, "a tall column drops nothing");
    // Anchored to the bottom edge: 1+1+3+2+2 value rows and three gaps —
    // the sky has none, it rides under the clock.
    let (_, bottom) = span(top, &shown, &f);
    assert_eq!(bottom, 48, "the foot ends on the last row");
    assert_eq!(top, 48 - 12, "and starts its own height above it");
}

#[test]
fn the_foot_never_stands_on_a_pane_row_and_never_touches_one() {
    // 12 panes on a 20-row rail: the list is served first, and a blank row
    // always stands between the last pane and the first reading.
    let f = full(None);
    let (top, shown) = plan(&f, 20, 12);
    assert!(!shown.is_empty(), "there was still room for something");
    assert!(top > 12, "the foot started below the last pane row: {top}");
}

#[test]
fn a_short_column_gives_up_the_sky_then_the_rates_and_keeps_the_clock_last() {
    let f = full(Some(git_at(0, 0, 0)));
    // Room for 10 of the 12 rows: the sky goes, then the rates.
    let (_, shown) = plan(&f, 11, 0);
    assert!(
        !shown.contains(&Group::Sky),
        "the sky went first: {shown:?}"
    );
    assert!(shown.contains(&Group::Sys), "{shown:?}");
    let (_, shown) = plan(&f, 9, 0);
    assert!(
        !shown.contains(&Group::Net),
        "the rates went next: {shown:?}"
    );
    // One row of room: the clock is what a rail that short still shows.
    assert_eq!(plan(&f, 2, 0).1, vec![Group::Time]);
    // None at all: nothing is drawn rather than something clipped.
    assert!(plan(&f, 1, 0).1.is_empty(), "no room, no foot");
}

#[test]
fn a_reading_with_nothing_behind_it_reserves_no_rows() {
    let mut f = full(None);
    f.sky = None;
    let (_, shown) = plan(&f, 48, 1);
    assert!(!shown.contains(&Group::Git), "not a repo: {shown:?}");
    assert!(!shown.contains(&Group::Sky), "no place set: {shown:?}");
    // What is left is 1 + 3 + 2 value rows and two gaps — the dropped
    // sections took their gaps with them.
    assert_eq!(height(&shown, &f), 8, "{shown:?}");
}

#[test]
fn the_ahead_behind_row_exists_only_when_there_is_something_to_say() {
    assert_eq!(Group::Git.rows(&full(Some(git_at(2, 0, 0)))), 1);
    assert_eq!(Group::Git.rows(&full(Some(git_at(2, 1, 0)))), 2);
    assert_eq!(Group::Git.rows(&full(Some(git_at(2, 0, 3)))), 2);
}

#[test]
fn every_group_draws_inside_the_rows_the_plan_gave_it() {
    let f = full(Some(git_at(4, 2, 1)));
    let (rows, used) = (40u16, 3u16);
    let (top, shown) = plan(&f, rows, used);
    let (cells, paint) = foot(&f, 5, rows, used, 2.0);
    assert!(!cells.is_empty() && !paint.is_empty(), "the foot drew");
    for c in &cells {
        assert!(c.row >= top, "row {} is above the foot's top {top}", c.row);
        assert!(c.row < rows, "row {} fell off the card", c.row);
        assert!(c.col < 5, "col {} fell out of the rail", c.col);
    }
    let (_, bottom) = span(top, &shown, &f);
    assert_eq!(bottom, rows, "the last reading sits on the last row");
}

/// The glyphs of `row` in `cells`, left to right.
fn text(cells: &[CellView], row: u16) -> String {
    let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == row).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect()
}

#[test]
fn the_clock_is_drawn_in_full_at_five_columns_with_the_sky_under_it() {
    let f = full(None);
    let (cells, _) = foot(&f, 5, 40, 0, 2.0);
    let top = plan(&f, 40, 0).0;
    assert_eq!(
        text(&cells, top),
        "09:41",
        "hours and minutes both survived"
    );
    assert_eq!(
        text(&cells, top + 1),
        "\u{2601}12\u{00b0}",
        "the sky is on the very next row, as it is on the open nav"
    );
}

#[test]
fn a_blank_row_stands_between_every_two_groups() {
    let f = full(Some(git_at(1, 0, 0)));
    let (cells, paint) = foot(&f, 5, 40, 0, 2.0);
    let (top, shown) = plan(&f, 40, 0);
    let mut row = top;
    for (i, g) in shown.iter().enumerate() {
        if g.gap_before(i == 0) > 0 {
            let empty = text(&cells, row).trim().is_empty()
                && !paint
                    .iter()
                    .any(|p| p.y < f32::from(row) + 1.0 && p.y + p.h > f32::from(row));
            assert!(empty, "row {row} before {g:?} was not blank");
            row += 1;
        }
        row += g.rows(&f);
    }
}
