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

/// The panel stands over the panes' area, a row under its top and a gap
/// clear of the input bar: centred on the whole window it covered the bar
/// and sat 4 px off the panes' rules (glass survey D-M4).
#[test]
fn the_panel_stands_clear_of_the_input_bar() {
    let (cw, ch) = (8.0, 17.0);
    let mut app = CrewApp {
        geo_override: Some((cw, ch, 1280.0, 720.0, 1.0)),
        ..Default::default()
    };
    app.help_open = true;
    let scenes = app.build_frame();
    let (content, _) = app.placed_grid().expect("a frame");
    let bar = crate::chrome::inputbar_rect(content, 720.0, ch, app.gutter());
    let panel = scenes
        .iter()
        .filter(|s| s.overlay && s.glass)
        .max_by(|a, b| (a.w * a.h).total_cmp(&(b.w * b.h)))
        .expect("the keys panel");
    assert!(
        panel.y >= content.y + ch - 1.0,
        "a row under the top: {}",
        panel.y
    );
    let air = bar.y - (panel.y + panel.h);
    assert!(
        air >= app.gutter().y,
        "clear of the bar by the canvas's gap: {air}"
    );
    assert!(panel.x >= content.x, "right of the nav");
}
