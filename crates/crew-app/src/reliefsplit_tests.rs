use super::*;

fn grid(rows: &[&str]) -> Vec<CellView> {
    let mut v = Vec::new();
    for (r, line) in rows.iter().enumerate() {
        for (c, ch) in line.chars().enumerate() {
            v.push(CellView {
                col: c as u16,
                row: r as u16,
                c: ch,
                ..Default::default()
            });
        }
    }
    v
}

fn text(cells: &[CellView], row: u16) -> String {
    let mut r: Vec<_> = cells.iter().filter(|c| c.row == row).collect();
    r.sort_by_key(|c| c.col);
    r.iter().map(|c| c.c).collect()
}

const FAR: &[&str] = &[
    "╭ a ──┬ b ──╮",
    "│ x   │ y ┬ │",
    "│ ─┴─ │     │",
    "╰─────┴─────╯",
];
const BOX: Box4 = Box4 {
    c0: 0,
    r0: 0,
    c1: 12,
    r1: 3,
};

/// A box is divided where a tee on its top edge stands over one on its
/// bottom; a tee inside the panels is their own drawing, not a joint.
#[test]
fn a_joint_is_a_tee_on_the_top_edge_over_one_on_the_bottom() {
    assert_eq!(joints(&grid(FAR), &BOX), vec![6]);
    let plain = ["╭────╮", "│    │", "╰────╯"];
    let b = Box4 {
        c0: 0,
        r0: 0,
        c1: 5,
        r1: 2,
    };
    assert!(joints(&grid(&plain), &b).is_empty());
}

/// Each panel is its own box: the first stops a column short of the rule,
/// the second starts on it. A box with no joint is one panel, itself.
#[test]
fn a_divided_box_is_a_panel_each_side_of_its_rule() {
    let p = panels(&BOX, &[6]);
    assert_eq!(p, vec![Box4 { c1: 5, ..BOX }, Box4 { c0: 6, ..BOX }]);
    assert_eq!(panels(&BOX, &[]), vec![BOX]);
}

/// The dividing rule goes, edge to edge, and nothing else in the panels.
#[test]
fn the_rule_between_the_panels_is_blanked_and_only_it() {
    let mut cells = grid(FAR);
    undivide(&mut cells, &BOX, &[6]);
    assert_eq!(text(&cells, 0), "╭ a ──  b ──╮");
    assert_eq!(text(&cells, 1), "│ x     y ┬ │");
    assert_eq!(text(&cells, 2), "│ ─┴─       │");
    assert_eq!(text(&cells, 3), "╰───── ─────╯");
}
