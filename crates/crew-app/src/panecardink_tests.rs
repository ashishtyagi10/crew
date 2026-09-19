use super::*;

fn bar<'a>(focused: bool, broadcast: bool) -> Bar<'a> {
    Bar {
        index: Some(1),
        title: "zsh",
        focused,
        broadcast,
        ..Default::default()
    }
}

/// The loud one: while broadcast is on, every pane it reaches wears the
/// broadcast colour — focused or not, so the group reads as a group.
#[test]
fn broadcast_paints_the_whole_frame_on_every_pane_it_reaches() {
    let _g = crate::app::theme_test_guard();
    let cast = crew_theme::theme().broadcast;
    let hue = (10, 20, 30);
    assert_eq!(stroke(&bar(false, true), hue).0, cast);
    assert_eq!(stroke(&bar(true, true), hue).0, cast);
}

/// …and off, nothing changes: the focused card keeps its bright stroke and
/// the rest keep the normal one.
#[test]
fn without_broadcast_the_frame_is_the_one_it_always_was() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    let hue = (10, 20, 30);
    assert_eq!(
        stroke(&bar(true, false), hue).0,
        crate::panecardglow::focused_stroke(t)
    );
    assert_eq!(stroke(&bar(false, false), hue).0, t.border_normal);
}

/// The legend keeps saying WHICH pane this is in either mode — the frame
/// carries the mode, the legend carries the identity.
#[test]
fn the_legend_keeps_the_panes_own_hue() {
    let _g = crate::app::theme_test_guard();
    let hue = (200, 40, 90);
    assert_eq!(stroke(&bar(true, true), hue).1, hue);
    assert_eq!(stroke(&bar(true, false), hue).1, hue);
    let dim = stroke(&bar(false, true), hue).1;
    assert_ne!(dim, hue, "an unfocused legend recedes");
    assert_eq!(dim, stroke(&bar(false, false), hue).1);
}
