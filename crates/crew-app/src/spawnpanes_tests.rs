//! Which of crew's own panes open a second copy, and which come back.
use crate::app::CrewApp;

#[test]
fn escape_puts_the_todo_list_away_rather_than_throwing_it_out() {
    // The todo list is the one crew-drawn surface you keep open while you
    // work, so Esc minimizes it into the nav and `/todo` brings it back —
    // with whatever view and filter it had.
    let mut app = CrewApp::default();
    assert!(!app.submit_input("/todo".to_string()));
    assert_eq!(app.panes.len(), 1);

    app.minimize_pane(0);
    assert!(app.panes[0].hidden, "not minimized");
    assert_eq!(app.panes.len(), 1, "minimizing must not close it");

    // Opening it again is how you get it back — not a second list.
    assert!(!app.submit_input("/todo".to_string()));
    assert_eq!(app.panes.len(), 1, "a second todo pane was opened");
    assert_eq!(app.focused, 0);

    // The visit surfaces are unchanged: they still stack.
    assert!(!app.submit_input("/far".to_string()));
    assert!(!app.submit_input("/far".to_string()));
    assert_eq!(app.panes.len(), 3, "far stopped stacking");
}
