use super::*;
use crate::cmdcheck::Verdict;

#[test]
fn idle_shell_wins_over_everything() {
    // Even a resolvable command goes INTO an idle focused shell.
    let r = route_bare(Target::IdleShell(2), &Verdict::Executable("ls".into()));
    assert!(matches!(r, BareRoute::TypeInto(2)));
    // …and so does prose: the shell is the judge of what it means.
    let r = route_bare(Target::IdleShell(0), &Verdict::No);
    assert!(matches!(r, BareRoute::TypeInto(0)));
}

#[test]
fn busy_or_nonterminal_focus_diverts_by_verdict() {
    assert!(matches!(
        route_bare(Target::Other, &Verdict::Executable("claude".into())),
        BareRoute::Spawn
    ));
    assert!(matches!(
        route_bare(Target::Other, &Verdict::Builtin("export".into())),
        BareRoute::BuiltinHint(b) if b == "export"
    ));
    assert!(matches!(
        route_bare(Target::Other, &Verdict::No),
        BareRoute::UnknownHint
    ));
}

/// A Far pane, just enough to seed `CrewApp::panes` for preview tests
/// (mirrors the identically-named private helpers in panemanage.rs,
/// navcard.rs, and app_tests.rs — no shared helper exists yet, so each
/// test module keeps its own).
#[cfg(unix)]
fn far_pane(name: &str) -> crate::pane::Pane {
    use crate::farpane::FarPane;
    use crate::pane::{Pane, PaneContent};
    use crew_term::GridSize;
    Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Far(FarPane::new(std::env::temp_dir())),
        grid: GridSize { cols: 80, rows: 24 },
        rect: crate::layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: Some(name.to_string()),
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: crate::anim::now_ms(),
    }
}

// Depends on `ls` existing on PATH, which is a POSIX assumption — on
// Windows this asserts the platform's command set, not crew's routing.
#[cfg(unix)]
#[test]
fn preview_labels_spawn_and_hint_rows() {
    let mut app = crate::app::CrewApp::default();
    app.panes.push(far_pane("files"));
    app.focused = 0;
    // Resolvable → a submit row naming the new pane destination.
    app.input.text = "ls".into();
    let rows = app.input_preview();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].label.contains("new pane"), "got: {}", rows[0].label);
    assert!(rows[0].submit);
    // Unresolvable → a dim non-submit hint row.
    app.input.text = "definitely-not-a-command-xyz".into();
    let rows = app.input_preview();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].submit);
    assert!(
        rows[0].label.contains("not a command"),
        "got: {}",
        rows[0].label
    );
}

#[test]
fn preview_is_silent_for_slash_cd_and_empty() {
    let mut app = crate::app::CrewApp::default();
    app.input.text = "/theme".into();
    assert!(app.input_preview().is_empty(), "slash palette owns / input");
    app.input.text = "cd ~/code".into();
    assert!(
        app.input_preview().is_empty(),
        "cd keeps its ghost, no card"
    );
    app.input.text = String::new();
    assert!(app.input_preview().is_empty());
}

#[test]
fn preview_is_silent_for_unrecognized_slash_command() {
    // `/definitely-not-a-palette-cmd` matches no slash-palette row, but
    // `submit_input` still routes it to slash dispatch (which silently
    // no-ops) — never to route_bare. The preview must not show a
    // submit-labeled spawn/type-into row Enter will never honor.
    let mut app = crate::app::CrewApp::default();
    app.input.text = "/definitely-not-a-palette-cmd".into();
    assert!(
        app.input_preview().is_empty(),
        "slash dispatch owns all /-led text, even unrecognized commands"
    );
}

#[test]
fn preview_shows_the_ask_row_for_qmark_text() {
    let mut app = crate::app::CrewApp::default();
    app.input.text = "?list rust files".into();
    let rows = app.input_preview();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].submit, "a filled ?query submits");
    assert!(rows[0].label.contains("ask ai"), "got: {}", rows[0].label);
    // Bare `?` mirrors the usage hint, like bare `!` and `*`.
    app.input.text = "?".into();
    let rows = app.input_preview();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].submit);
    assert!(rows[0].label.contains("usage"), "got: {}", rows[0].label);
}

#[test]
fn preview_counts_broadcast_targets() {
    let mut app = crate::app::CrewApp::default();
    app.input.text = "* echo hi".into();
    let rows = app.input_preview();
    assert_eq!(rows.len(), 1);
    assert!(
        rows[0].label.contains("0 terminals"),
        "got: {}",
        rows[0].label
    );
}

#[test]
fn bare_prefixes_show_usage_hint_not_submit_row() {
    // Bare `!` and `*` (empty payload) submit to a usage-hint status in
    // submit_input, not a spawn/broadcast — the preview must mirror that
    // with a single non-submit row, not a submit-labeled one Enter will
    // never honor.
    let mut app = crate::app::CrewApp::default();
    app.input.text = "!".into();
    let rows = app.input_preview();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].submit);
    assert_eq!(rows[0].label, "usage: !<command>");

    app.input.text = "*".into();
    let rows = app.input_preview();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].submit);
    assert_eq!(rows[0].label, "usage: *<text> — sends to every terminal");
}
