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
