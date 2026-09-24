use super::*;

fn rect() -> Rect {
    Rect {
        x: 10.0,
        y: 10.0,
        w: 400.0,
        h: 300.0,
    }
}

/// The glass scene — the one the sheet, its shadow and its rim are drawn
/// under.
fn sheet(scenes: &[PaneScene]) -> &PaneScene {
    scenes.iter().find(|s| s.glass).expect("a card has a sheet")
}

/// Every other panel card stays on the page: a nav full of risen cards has
/// no focus at all.
#[test]
fn a_plain_card_rests_on_the_page() {
    let mut scenes = Vec::new();
    push_card(&mut scenes, rect(), 8.0, 16.0, "todo", |_, _| Vec::new());
    let s = sheet(&scenes);
    assert_eq!(s.lift, 0.0);
    assert!(!s.focused);
}
