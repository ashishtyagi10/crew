use super::*;

fn corner(row: u16, col: u16, c: char, fg: (u8, u8, u8)) -> CellView {
    CellView {
        row,
        col,
        c,
        fg,
        ..Default::default()
    }
}

const ACCENT: (u8, u8, u8) = (67, 72, 190);
const RULE: (u8, u8, u8) = (180, 180, 184);

/// Two tab cards, the second framed in the accent.
fn two() -> (Vec<CellView>, Vec<Box4>) {
    let mut v = Vec::new();
    for (r0, fg) in [(1, RULE), (5, ACCENT)] {
        v.push(corner(r0, 1, '\u{256d}', fg));
        v.push(corner(r0, 20, '\u{256e}', fg));
        v.push(corner(r0 + 2, 1, '\u{2570}', fg));
        v.push(corner(r0 + 2, 20, '\u{256f}', fg));
    }
    let tabs = crate::reliefwell::boxes(&v);
    (v, tabs)
}

#[test]
fn the_card_framed_in_the_accent_is_the_active_tab() {
    let (v, tabs) = two();
    assert_eq!(tabs.len(), 2);
    let on = active(&v, &tabs, ACCENT).expect("an active tab");
    assert_eq!(on.r0, 5);
    assert_eq!(
        active(&v, &tabs, (1, 2, 3)),
        None,
        "no accent, no active tab"
    );
}

#[test]
fn the_active_tab_stands_up_and_the_rest_stay_sunk() {
    assert!(depth(true) > 0.0, "raised");
    assert_eq!(depth(false), DEPTH);
}
