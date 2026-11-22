//! How the tree draws.
use super::*;
use crate::explorer::Explorer;

fn rows_of(cells: &[CellView]) -> Vec<String> {
    let max = cells.iter().map(|c| c.row).max().unwrap_or(0);
    (0..=max)
        .map(|r| {
            let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == r).collect();
            v.sort_by_key(|c| c.col);
            let mut s = String::new();
            for c in v {
                while s.chars().count() < usize::from(c.col) {
                    s.push(' ');
                }
                s.push(c.c);
            }
            s.trim_end().to_string()
        })
        .collect()
}

#[test]
fn a_tree_draws_with_chevrons_guides_and_a_capitalised_root() {
    let _g = crate::app::theme_test_guard();
    let root = crate::explorer::tests::tree("draw", &["src/a.rs", "src/deep/b.rs", "x.md"]);
    let mut e = Explorer::default();
    let items = e.items(&root);
    e.toggle(&items[1]);
    let items = e.items(&root);
    e.toggle(&items[2]); // src/deep
    let items = e.items(&root);
    let drawn = rows_of(&cells(&items, 0, None, None, (30, 20)));
    assert!(drawn[0].contains("EXPLORER"), "{drawn:?}");
    assert!(
        drawn[1].starts_with(" \u{25be} CREW-EXPLORER-DRAW"),
        "{drawn:?}"
    );
    assert_eq!(drawn[2], "   \u{25be} src");
    assert_eq!(drawn[3], "   \u{2502} \u{25be} deep");
    assert_eq!(drawn[4], "   \u{2502} \u{2502}   b.rs");
    assert_eq!(drawn[5], "   \u{2502}   a.rs");
    assert_eq!(drawn[6], "     x.md");
}

/// Scrolled, item `k` is on row `k - scroll + TOP`, and nothing is drawn past the panel.
#[test]
fn a_scrolled_tree_starts_at_its_scroll_and_stops_at_the_panel() {
    let _g = crate::app::theme_test_guard();
    let names: Vec<String> = (0..30).map(|i| format!("f{i:02}")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let root = crate::explorer::tests::tree("scroll", &refs);
    let items = Explorer::default().items(&root);
    let c = cells(&items, 5, None, None, (20, 10));
    let drawn = rows_of(&c);
    assert_eq!(drawn[1].trim(), items[5].name);
    assert!(c.iter().all(|c| c.row < 10 && c.col < 20));
}
