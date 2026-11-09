use super::*;

fn row(index: usize, title: &str) -> PaneRow {
    PaneRow {
        index,
        title: title.into(),
        focused: false,
        activity: false,
        minimized: false,
        attention: None,
        busy: false,
        unread: 0,
        hovered: false,
    }
}

fn block(num: usize, title: &str, active: bool, rows: Vec<PaneRow>) -> TabBlock {
    TabBlock {
        num,
        title: title.into(),
        active,
        hovered: false,
        rows,
    }
}

/// The text of `row`, columns in order (gaps as spaces).
fn line(cells: &[CellView], row: u16, cols: u16) -> String {
    let mut s = vec![' '; usize::from(cols)];
    for c in cells.iter().filter(|c| c.row == row && c.col < cols) {
        s[usize::from(c.col)] = c.c;
    }
    s.into_iter().collect::<String>().trim_end().to_string()
}

fn list() -> NavList {
    NavList::Tabs(vec![
        block(1, "build", true, vec![row(1, "cargo"), row(2, "logs")]),
        block(2, "web", false, vec![row(3, "vite")]),
    ])
}

#[test]
fn each_tab_is_a_titled_card_with_its_panes_inside() {
    let cells = list().cells(30, 20, ' ');
    assert!(line(&cells, 0, 30).contains("PANES"));
    let top = line(&cells, 1, 30);
    assert!(top.starts_with(" \u{256d}\u{2500} 1 \u{b7} build"), "{top}");
    assert!(
        top.ends_with("2 \u{2500}\u{256e}"),
        "the pane count rides the edge: {top}"
    );
    let pane = line(&cells, 2, 30);
    assert!(
        pane.starts_with(" \u{2502}") && pane.ends_with('\u{2502}'),
        "{pane}"
    );
    assert!(pane.contains("1 cargo"), "{pane}");
    assert!(line(&cells, 4, 30).starts_with(" \u{2570}"), "bottom edge");
    assert!(line(&cells, 5, 30).contains("2 \u{b7} web"));
    assert!(line(&cells, 6, 30).contains("3 vite"));
    assert_eq!(list().len(), 7);
}

#[test]
fn the_tab_on_screen_is_framed_in_the_accent_and_the_rest_quietly() {
    let _g = crate::appearanceguard::theme_test_guard();
    let cells = list().cells(30, 20, ' ');
    let corner = |row| {
        cells
            .iter()
            .find(|c| c.row == row && c.col == 1)
            .map(|c| c.fg)
    };
    assert_eq!(corner(1), Some(crate::palette::accent()));
    assert_eq!(corner(5), Some(crew_theme::theme().border_normal));
}

#[test]
fn a_tab_behind_with_a_pane_that_needs_you_takes_the_bells_colour() {
    let _g = crate::appearanceguard::theme_test_guard();
    let mut r = row(3, "vite");
    r.attention = Some(('!', true));
    let l = NavList::Tabs(vec![
        block(1, "build", true, vec![row(1, "cargo")]),
        block(2, "web", false, vec![r]),
    ]);
    let cells = l.cells(30, 20, ' ');
    let corner = cells
        .iter()
        .find(|c| c.row == 4 && c.col == 1)
        .map(|c| c.fg);
    assert_eq!(corner, Some(crew_theme::theme().bell));
}

#[test]
fn the_list_stops_at_its_limit() {
    let cells = list().cells(30, 3, ' ');
    assert!(
        cells.iter().all(|c| c.row <= 3),
        "nothing past the third line"
    );
}

#[test]
fn the_rail_marks_each_tab_with_a_numbered_divider() {
    let cells = list().rail_cells(5, 20, ' ');
    assert_eq!(line(&cells, 0, 5), "\u{2500}1\u{2500}\u{2500}\u{2500}");
    assert!(line(&cells, 1, 5).contains('1'));
    assert_eq!(line(&cells, 3, 5), "\u{2500}2\u{2500}\u{2500}\u{2500}");
    assert_eq!(list().rail_len(), 5);
}

#[test]
fn one_tab_draws_the_flat_list_it_always_did() {
    let flat: NavList = vec![row(1, "cargo")].into();
    let want = crate::panelist::pane_cells(&[row(1, "cargo")], 30, 5, ' ');
    assert_eq!(flat.cells(30, 5, ' ').len(), want.len());
    assert_eq!(flat.len(), 1);
}

fn three_panes(title: &str) -> NavList {
    let rows = vec![row(1, "a"), row(2, "b"), row(3, "c")];
    NavList::Tabs(vec![
        block(1, title, true, rows),
        block(2, "x", false, vec![]),
    ])
}

#[test]
fn a_long_tab_name_keeps_its_ellipsis_and_the_count_its_place() {
    // 40 columns of name on a 24-column nav: `· {name} ` is three wider
    // than the name, so clipping to the gap alone cut the `…` off.
    let l = three_panes("web frontend staging server for the team");
    let top = line(&l.cells(24, 20, ' '), 1, 24);
    assert!(top.contains('\u{2026}'), "the name says it was cut: {top}");
    assert!(top.ends_with(" 3 \u{2500}\u{256e}"), "{top}");
}

#[test]
fn a_narrow_nav_keeps_the_pane_count_and_lets_the_name_go() {
    let l = three_panes("build");
    let top = line(&l.cells(16, 20, ' '), 1, 16);
    assert!(
        top.ends_with(" 3 \u{2500}\u{256e}"),
        "the count stays: {top}"
    );
    assert!(top.contains(" 1 "), "{top}");
}

#[test]
fn a_cut_list_says_how_many_panes_it_shows() {
    // Three lines hold tab 1's edge and two rows: pane 3 (tab 2) is hidden.
    let key = line(&list().cells(30, 3, ' '), 0, 30);
    assert!(key.contains("2 tabs \u{b7} 2/3"), "{key}");
    let all = line(&list().cells(30, 20, ' '), 0, 30);
    assert!(all.contains("2 tabs \u{b7} 3"), "{all}");
}

#[test]
fn the_rails_divider_takes_the_bell_for_a_tab_that_needs_you() {
    let _g = crate::appearanceguard::theme_test_guard();
    let mut r = row(3, "vite");
    r.attention = Some(('!', true));
    let l = NavList::Tabs(vec![
        block(1, "build", true, vec![row(1, "cargo")]),
        block(2, "web", false, vec![r]),
    ]);
    let cells = l.rail_cells(5, 20, ' ');
    let bell = crew_theme::theme().bell;
    let divider: Vec<_> = cells.iter().filter(|c| c.row == 2).collect();
    assert!(!divider.is_empty() && divider.iter().all(|c| c.fg == bell));
}
