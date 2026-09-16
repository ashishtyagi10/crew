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

#[test]
fn the_visit_surfaces_open_zoomed_and_esc_lands_you_back_on_the_grid() {
    // A visit wants the window: far has two panes of its own to fit, and a
    // settings form is a column to read down.
    for cmd in ["/settings", "/far"] {
        let mut app = CrewApp::default();
        assert!(
            !app.submit_input("/todo".to_string()),
            "a pane to come back to"
        );
        assert!(!app.zoomed, "the todo list is not a visit");

        assert!(!app.submit_input(cmd.to_string()));
        assert!(app.zoomed, "{cmd} did not open zoomed");

        // Leaving is what un-zooms — `close_pane` already clears it, which is
        // why this needed no new key.
        let focused = app.focused;
        app.close_pane(focused);
        assert!(!app.zoomed, "{cmd} left the app zoomed after closing");
        assert_eq!(app.panes.len(), 1, "back to the grid we came from");
    }
}

#[test]
fn opening_a_visit_over_a_zoomed_pane_zooms_the_new_one() {
    // `zoomed` means "the FOCUSED pane is zoomed", so opening a second visit
    // must leave the flag set on the pane you are now looking at.
    let mut app = CrewApp::default();
    assert!(!app.submit_input("/far".to_string()));
    assert!(!app.submit_input("/settings".to_string()));
    assert!(app.zoomed);
    assert_eq!(app.focused, app.panes.len() - 1, "focus is on the new pane");
}
