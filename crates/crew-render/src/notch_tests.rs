use super::*;

/// A row of `text` laid from column 0, the way a frame's cells spell it.
fn row(row: u16, text: &str) -> Vec<CellView> {
    text.chars()
        .enumerate()
        .map(|(col, c)| CellView {
            col: col as u16,
            row,
            c,
            ..Default::default()
        })
        .collect()
}

/// `╭─ 2 far ──[-][x]─╮`: the title and the buttons are two gaps in the
/// rule, and each notch spans exactly the columns the rule is missing from —
/// the padding spaces included, since the stroke stops there too.
#[test]
fn legends_notch_the_top_edge_where_the_rule_stops() {
    let top = "╭─ 2 far ──[-][x]─╮";
    let cols = top.chars().count();
    let mut cells = row(0, top);
    cells.extend(row(3, &format!("╰{}╯", "─".repeat(cols - 2))));
    let n = notch(&cells, cols, 4, 10.0, 20.0, 5.0, 10.0);
    assert_eq!(n.top[0], [2.0 * 10.0 - 5.0, 9.0 * 10.0 - 5.0], "' 2 far '");
    assert_eq!(n.top[1], [11.0 * 10.0 - 5.0, 17.0 * 10.0 - 5.0], "[-][x]");
    assert_eq!(n.top[2], [0.0, 0.0], "nothing else");
    assert_eq!(n.bottom, [[0.0; 2]; SPANS], "an unbroken bottom rule");
    assert_eq!(n.depth, 10.0, "the legend row reaches half a cell in");
}

/// A status written into the bottom rule notches the bottom edge.
#[test]
fn a_bottom_legend_notches_the_bottom_edge() {
    let mut cells = row(0, "╭────────╮");
    cells.extend(row(2, "╰── far ─╯"));
    let n = notch(&cells, 10, 3, 10.0, 20.0, 0.0, 10.0);
    assert_eq!(n.top, [[0.0; 2]; SPANS]);
    assert_eq!(n.bottom[0], [30.0, 80.0]);
}

/// A rule not yet drawn (a frame assembling) is bare, not a legend: no ink,
/// no notch — the rim follows the light, not the animation's gaps.
#[test]
fn a_bare_stretch_without_ink_is_no_notch() {
    let cells = row(0, "╭──      ──╮");
    let n = notch(&cells, 12, 3, 10.0, 20.0, 0.0, 10.0);
    assert_eq!(n.top, [[0.0; 2]; SPANS]);
}

/// More legends than the shader has slots for merge across their NARROWEST
/// rule, so the widest stretches of lit rim survive.
#[test]
fn surplus_legends_merge_across_the_narrowest_rule() {
    let top = "╭─a─b────c─d─e─────f─╮";
    let cols = top.chars().count();
    let n = notch(&row(0, top), cols, 3, 1.0, 20.0, 0.0, 10.0);
    let spans: Vec<[f32; 2]> = n.top.iter().copied().filter(|s| s[1] > s[0]).collect();
    assert_eq!(spans.len(), SPANS);
    assert_eq!(spans[0], [2.0, 5.0], "a and b bridged");
    assert_eq!(spans[1], [9.0, 12.0], "c and d bridged");
}
