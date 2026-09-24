use super::*;

fn row(kind: Wait, text: &str) -> WaitRow {
    WaitRow {
        kind,
        text: text.into(),
        pane: Some(0),
        hovered: false,
    }
}

fn line(cells: &[CellView], r: u16) -> String {
    let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == r).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect::<String>().trim().to_string()
}

/// The rule carries the count of real rows, each row wears its kind's
/// colour, the list stops at `max_lines`, and a quiet card counts nothing.
#[test]
fn rows_wear_their_kind_and_the_rule_counts_them() {
    let _g = crate::palette::test_guard();
    let _t = crate::app::theme_test_guard();
    let rows = vec![
        row(Wait::Blocked, "\u{2691} zsh"),
        row(Wait::Plan, "\u{21b5} plan \u{00b7} smith"),
        row(Wait::Running, "\u{25b6} 2 running \u{00b7} smith"),
    ];
    let cells = waiting_cells(&rows, 40, 2);
    assert!(line(&cells, 0).contains("WAITING ON YOU"));
    assert!(line(&cells, 0).contains('3'), "{}", line(&cells, 0));
    assert_eq!(line(&cells, 1), "\u{2691} zsh");
    assert_eq!(line(&cells, 2), "\u{21b5} plan \u{00b7} smith");
    assert!(cells.iter().all(|c| c.row <= 2), "capped at max_lines");
    let t = crew_theme::theme();
    assert!(cells.iter().any(|c| c.row == 1 && c.fg == t.bell));
    assert!(cells.iter().any(|c| c.row == 2 && c.fg == t.status_fg));
    let quiet = waiting_cells(&[row(Wait::Quiet, "nothing")], 40, 3);
    assert!(!line(&quiet, 0).chars().any(|c| c.is_ascii_digit()));
    assert!(waiting_cells(&rows, 40, 0).is_empty());
    let narrow = waiting_cells(&rows, 18, 2);
    assert!(line(&narrow, 0).contains("WAITING") && !line(&narrow, 0).contains('\u{2026}'));
}

/// The row under the pointer lifts; the others stay as they were.
#[test]
fn the_hovered_row_is_bold_and_the_rest_are_not() {
    let _g = crate::app::theme_test_guard();
    let mut rows = vec![row(Wait::Blocked, "a"), row(Wait::Plan, "b")];
    rows[1].hovered = true;
    let cells = waiting_cells(&rows, 30, 4);
    assert!(
        cells.iter().filter(|c| c.row == 2).all(|c| c.bold),
        "hovered row"
    );
    assert!(
        cells.iter().filter(|c| c.row == 1).all(|c| !c.bold),
        "the other row"
    );
}

/// Every row's WORDS start under the rule's legend, as every other nav
/// card's do; a row's icon hangs in the margin before them.
#[test]
fn words_start_under_the_legend_and_icons_hang() {
    let rows = vec![
        row(Wait::Blocked, "\u{2691} zsh"),
        row(Wait::Quiet, "nothing \u{2014} all quiet"),
    ];
    let cells = waiting_cells(&rows, 40, 2);
    let at = |r: u16, c: char| cells.iter().find(|x| x.row == r && x.c == c).map(|x| x.col);
    let indent = crate::navtext::INDENT;
    assert_eq!(at(1, 'z'), Some(indent), "the pane's name");
    assert_eq!(at(1, '\u{2691}'), Some(indent - 2), "the flag hangs");
    assert_eq!(at(2, 'n'), Some(indent), "the quiet row");
}
