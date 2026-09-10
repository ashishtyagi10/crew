//! Two nav rules, one convention: a section that shows fewer rows than it
//! has says `shown/total` on its rule. The LOG did; the PANES rule said the
//! total alone, so twelve panes over five rows read as a list of five.
use super::depth_key;

#[test]
fn a_section_that_fits_says_its_count_and_one_that_does_not_says_shown_of_total() {
    assert_eq!(depth_key(3, 3), "3");
    assert_eq!(depth_key(5, 12), "5/12");
    assert_eq!(depth_key(0, 0), "0");
}

fn row(i: usize) -> crate::panelist::PaneRow {
    crate::panelist::PaneRow {
        index: i,
        title: "sh".into(),
        focused: false,
        activity: false,
        minimized: false,
        attention: None,
        busy: false,
        hovered: false,
        unread: 0,
    }
}

fn rule(panes: &[crate::panelist::PaneRow], limit: usize) -> String {
    let _g = crate::app::theme_test_guard();
    let cells = crate::panelist::pane_cells(panes, 30, limit, '⠋');
    let mut v: Vec<&crew_render::CellView> = cells.iter().filter(|c| c.row == 0).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect()
}

#[test]
fn the_panes_rule_says_shown_of_total_when_the_list_is_cut() {
    let panes: Vec<_> = (1..=12).map(row).collect();
    assert!(rule(&panes, 5).contains("5/12"), "{}", rule(&panes, 5));
    assert!(
        rule(&panes[..3], 5).contains(" 3 "),
        "{}",
        rule(&panes[..3], 5)
    );
    assert!(!rule(&panes[..3], 5).contains('/'));
}
