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

/// The welcome stands in for a lone focused terminal, so its sheet rides at a
/// focused pane's rise. It shipped lit-stroked but flat (lift 0), which on a
/// sheer window read as no card at all.
#[test]
fn the_lit_card_rises_like_a_focused_pane() {
    let mut scenes = Vec::new();
    push_card_lit(&mut scenes, rect(), 8.0, 16.0, "crew", |_, _| Vec::new());
    let s = sheet(&scenes);
    assert_eq!(s.lift, crate::spotlight::lift_for(0, 0, 0, 1.0));
    assert!(s.focused);
}
