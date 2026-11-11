use crate::app::CrewApp;
use crate::restartsnap::Win;
use crate::sessionsave::SavedPane;

/// Far panes: restorable, and nothing is spawned to make one.
fn far() -> SavedPane {
    SavedPane::far(std::env::temp_dir().to_string_lossy().into_owned())
}

fn app_with(n: usize) -> CrewApp {
    let mut app = CrewApp::default();
    app.restore_from(vec![far(); n]);
    assert_eq!(app.panes.len(), n);
    app
}

/// The round trip a restart makes: what one process hands over is what the
/// next one stands up — every pane (two in one directory stay two), and the
/// focus on the same one.
#[test]
fn a_window_comes_back_with_its_panes_and_its_focus() {
    let mut old = app_with(3);
    old.focused = 1;
    old.input.focused = false;
    let win = old.restart_window();
    assert_eq!(win.panes.len(), 3);
    assert_eq!(win.focus, Some(1));

    let mut new = CrewApp::default();
    assert_eq!(new.resume_window(win), 3);
    assert_eq!(new.panes.len(), 3);
    assert_eq!(new.focused, 1);
    assert!(!new.input.focused);
}

/// Typing `/restart` means the input bar had the focus — and it has it again.
#[test]
fn focus_on_the_input_bar_returns_to_the_input_bar() {
    let mut old = app_with(2);
    old.input.focused = true;
    let win = old.restart_window();
    assert_eq!(win.focus, None);

    let mut new = CrewApp::default();
    new.resume_window(win);
    assert!(new.input.focused);
}

#[test]
fn a_minimized_pane_comes_back_minimized() {
    let mut old = app_with(2);
    old.panes[0].hidden = true;
    let win = old.restart_window();
    let mut new = CrewApp::default();
    new.resume_window(win);
    assert!(new.panes[0].hidden);
    assert!(!new.panes[1].hidden);
}

/// A focus index is only trusted when every pane opened: after a failed
/// spawn it would name a neighbour.
#[test]
fn a_pane_that_fails_to_open_does_not_misplace_the_focus() {
    let mut new = CrewApp::default();
    let ghost = SavedPane {
        kind: "hologram".into(),
        ..Default::default()
    };
    let opened = new.resume_window(Win {
        focus: Some(2),
        panes: vec![far(), ghost, far()],
    });
    assert_eq!(opened, 2);
    assert!(new.focused < new.panes.len());
}

#[test]
fn an_empty_window_resumes_nothing() {
    let mut new = CrewApp::default();
    assert_eq!(new.resume_window(Win::default()), 0);
    assert!(new.panes.is_empty());
}

/// `/restart` and the nav's RESTART card only ASK: a canvas cannot snapshot
/// the other windows, so nothing is spawned and nothing exits from here.
#[test]
fn restart_is_an_ask_the_owner_answers() {
    let mut app = CrewApp::default();
    assert!(!app.want_restart);
    let exit = app.run_slash_command("restart");
    assert!(!exit, "the canvas does not exit by itself");
    assert!(app.want_restart);
}
