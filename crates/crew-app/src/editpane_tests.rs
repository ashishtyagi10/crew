use super::{edit_script, pick_editor, sh_quote};

#[test]
fn pick_editor_prefers_visual_then_editor_then_vi() {
    assert_eq!(
        pick_editor(Some("nvim".into()), Some("nano".into())),
        "nvim"
    );
    assert_eq!(pick_editor(None, Some("nano".into())), "nano");
    // blank values are ignored, falling through to the default.
    assert_eq!(pick_editor(Some("  ".into()), None), "vi");
    assert_eq!(pick_editor(None, None), "vi");
}

#[test]
fn sh_quote_escapes_spaces_and_quotes() {
    assert_eq!(sh_quote("a b.txt"), "'a b.txt'");
    assert_eq!(sh_quote("it's"), "'it'\\''s'");
}

#[test]
fn edit_script_quotes_path_and_keeps_pane_open() {
    let s = edit_script("code -w", "/tmp/a b.rs", "/bin/zsh");
    assert_eq!(s, "code -w '/tmp/a b.rs'; exec /bin/zsh");
}

// Drives a real PTY running a POSIX shell: Unix-only by construction.
// Windows has no `sh`, so the spawn fails on a detail that says nothing
// about the behaviour under test.
#[cfg(unix)]
#[test]
fn a_spawn_that_pushes_no_pane_does_not_adopt_an_unrelated_panes_born_ms() {
    // `edit_in_pane` returns without pushing a pane when given an empty
    // path — the same postcondition (`self.panes.len()` unchanged) that
    // a genuinely failed `PtyTerm` spawn leaves (`spawn.rs`'s `Err` arm),
    // reached deterministically rather than by relying on a shell that
    // can be made to fail on demand. An unrelated, already-running
    // terminal pane sits last in `self.panes` throughout: reading
    // `.last().born_ms` unconditionally in that case would adopt ITS
    // born_ms for the viewer, silently misdirecting
    // `reload_views_after_edit` at the wrong (unrelated, still-running)
    // pane instead of leaving `editor_born` untouched.
    use crate::app::{CrewApp, FALLBACK_SIZE};
    use crate::pane::{Pane, PaneContent, TermPane};
    use crate::spawn::PLACEHOLDER_RECT;
    use crew_term::PtyTerm;

    let dir = std::env::temp_dir();
    let f = dir.join("edit-guard.txt");
    std::fs::write(&f, "content\n").unwrap();
    let mut app = CrewApp {
        cwd: dir,
        ..Default::default()
    };
    app.open_view("edit-guard.txt");
    assert_eq!(app.panes.len(), 1, "the viewer must have opened");

    let pty = PtyTerm::spawn(FALLBACK_SIZE, "sh").expect("spawn an unrelated terminal");
    let input = pty.writer();
    let unrelated_born = 424_242;
    app.panes.push(Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Terminal(Box::new(TermPane {
            pty,
            input,
            cmd: Some("vim".to_string()),
            cmd_since: None,
            tail: Default::default(),
            read_at: 0,
            spans: Default::default(),
            trail: Default::default(),
            images: Default::default(),
        })),
        grid: FALLBACK_SIZE,
        rect: PLACEHOLDER_RECT,
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: unrelated_born,
    });

    let focused = 0; // the viewer
    app.apply_view_edit(focused, std::path::Path::new(""));

    match &app.panes[focused].content {
        // Fix 6: `assert_ne!(.., Some(unrelated_born))` passes for ANY
        // other value, including some other wrong pane's born_ms that
        // just isn't this specific one. The real postcondition is that
        // `editor_born` stays untouched — `None` — not merely "not this
        // one value".
        PaneContent::View(v) => assert_eq!(
            v.editor_born, None,
            "a spawn that pushed no pane must leave editor_born untouched"
        ),
        _ => panic!("still a viewer"),
    }
}
