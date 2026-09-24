use crate::panecard::{pane_card, Bar};

fn top(b: &Bar, cols: u16) -> String {
    let v = pane_card(cols, 6, b);
    let mut row: Vec<_> = v.iter().filter(|c| c.row == 0).collect();
    row.sort_by_key(|c| c.col);
    row.dedup_by_key(|c| c.col);
    row.iter().map(|c| c.c).collect()
}

/// A busy card's badges read like its legend: parted by `·`, a space either
/// side of the cluster, and no stroke running between them.
#[test]
fn badges_are_parted_by_dots_not_rule() {
    let _g = crate::app::theme_test_guard();
    let git = crate::git::GitInfo {
        branch: "main".into(),
        changed: 3,
        ahead: 0,
        behind: 0,
    };
    let b = Bar {
        title: "crew",
        git: Some(&git),
        elapsed: Some("1m02s".into()),
        unread: 8,
        activity: true,
        ..Default::default()
    };
    let row = top(&b, 60);
    assert!(
        row.contains(" main \u{25cf}3\u{b7}1m02s\u{b7}8\u{b7}\u{25cf} "),
        "{row:?}"
    );
}

/// A quiet card has nothing to seat, and its rule is untouched.
#[test]
fn a_quiet_card_keeps_its_rule() {
    let _g = crate::app::theme_test_guard();
    let b = Bar {
        title: "crew",
        ..Default::default()
    };
    assert!(!top(&b, 40).contains('\u{b7}'));
}
