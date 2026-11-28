//! Keys the open find bar must hand back, and when it must close (split out
//! of [`super`] for the line cap).
use super::*;

#[cfg(unix)]
#[test]
fn scroll_and_pane_walking_keys_pass_through_the_open_bar() {
    use winit::keyboard::{Key, ModifiersState as M, NamedKey};
    let (mut app, _) = app_with(&lines());
    assert!(app.term_find_open());
    for k in [
        NamedKey::PageUp,
        NamedKey::PageDown,
        NamedKey::Home,
        NamedKey::End,
    ] {
        assert!(
            !app.term_find_event(&Key::Named(k), true, M::SHIFT),
            "{k:?}"
        );
    }
    assert!(!app.term_find_event(&Key::Named(NamedKey::Tab), true, M::CONTROL));
    assert!(!app.term_find_event(&Key::Named(NamedKey::Tab), true, M::CONTROL | M::SHIFT));
    // Unshifted, they are still just eaten.
    assert!(app.term_find_event(&Key::Named(NamedKey::PageUp), true, M::empty()));
    assert!(app.term_find_event(&Key::Named(NamedKey::Tab), true, M::empty()));
}

#[cfg(unix)]
#[test]
fn the_bar_closes_when_focus_leaves_the_pane() {
    let (mut app, sent) = app_with("hello\r\n");
    let (mut second, _) = app_with("other\r\n");
    app.panes.push(second.panes.remove(0));
    assert!(app.term_find_open());
    app.focused = 1;
    app.push_focus();
    assert!(term(&app).find.is_none(), "the bar did not outlive focus");
    app.focused = 0;
    assert!(
        !app.term_find_input(FindKey::Char('a')),
        "back on the pane the keys are the program's"
    );
    // The input bar taking the keys closes it too.
    assert!(app.term_find_open());
    app.input.focused = true;
    app.close_find_off_focus();
    assert!(term(&app).find.is_none());
    assert!(sent.lock().unwrap().is_empty());
}
