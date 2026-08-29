//! The keys that open, walk and close the keys overlay.
use crate::app::CrewApp;
use winit::keyboard::{Key, NamedKey};

/// The keyboard-shortcuts panel spent its whole life with no keyboard
/// shortcut: `/keys`, typed into the bar, was the only way to it.
#[test]
fn the_shortcuts_panel_has_a_shortcut() {
    let mut app = CrewApp::default();
    assert!(!app.help_open);
    assert!(!app.handle_super_chord("/"), "Cmd+/ never exits the app");
    assert!(app.help_open, "Cmd+/ opens the keys overlay");
    // The shifted key arrives as its own character, exactly like `{` and `}`.
    app.close_help();
    app.handle_super_chord("?");
    assert!(app.help_open, "Cmd+? opens it too");
}

/// It opens on an unfiltered list at the top, however the last visit ended.
#[test]
fn it_opens_as_a_fresh_question() {
    let mut app = CrewApp::default();
    app.open_help();
    app.help_scroll = 12;
    app.help_filter = "pane".into();
    app.close_help();
    app.open_help();
    assert_eq!(app.help_scroll, 0);
    assert!(app.help_filter.is_empty());
}

/// Typing filters the list rather than dismissing it — with forty-odd
/// bindings, saying what you are looking for is the fastest way through.
#[test]
fn typing_still_filters() {
    let mut app = CrewApp::default();
    app.open_help();
    app.help_key(&Key::Character("p".into()));
    assert_eq!(app.help_filter, "p");
    assert!(app.help_open);
}

/// Arrows walk the list; every other plain key puts it away.
#[test]
fn arrows_walk_and_escape_closes() {
    let mut app = CrewApp::default();
    app.open_help();
    app.help_key(&Key::Named(NamedKey::ArrowDown));
    assert!(app.help_open, "an arrow scrolls rather than dismissing");
    app.help_key(&Key::Named(NamedKey::Escape));
    assert!(!app.help_open);
}
