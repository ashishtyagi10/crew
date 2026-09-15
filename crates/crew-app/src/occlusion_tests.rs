use super::*;

#[test]
fn a_hidden_window_is_asked_for_nothing_and_a_revealed_one_is_asked_once() {
    let mut app = CrewApp::default();
    assert!(app.wants_frame(), "a fresh window starts visible");
    app.set_occluded(true);
    assert!(!app.wants_frame(), "a hidden window still wanted frames");
    app.redraw(); // no window in a unit test: this must simply not panic
    app.set_occluded(false);
    assert!(app.wants_frame(), "the window came back and stayed silent");
}

/// The gate is one line inside `redraw`, and a later edit that drops it would
/// leave `occluded` set and read by nothing — green tests, hidden window,
/// frames again.
#[test]
fn redraw_asks_the_gate_rather_than_the_window_first() {
    let src = include_str!("occlusion.rs");
    let body = src
        .split("pub(crate) fn redraw(&self) {")
        .nth(1)
        .and_then(|b| b.split("\n    }").next())
        .expect("redraw moved");
    assert!(
        body.contains("wants_frame"),
        "redraw no longer consults the gate: {body}"
    );
    assert!(
        body.find("wants_frame") < body.find("request_redraw"),
        "the gate is checked after the frame is asked for: {body}"
    );
}
