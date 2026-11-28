use super::*;
use crate::config::CrewConfig;
use crate::settingspane::SettingsPane;

#[test]
fn dir_label_is_the_folder_name() {
    assert_eq!(
        dir_label(Path::new("/Users/atyagi/code/crew")),
        Some("crew".to_string())
    );
    assert_eq!(dir_label(Path::new("/")), Some("/".to_string()));
    assert_eq!(dir_label(Path::new("")), None);
}

#[test]
fn terminal_title_is_the_open_directory_folder() {
    let base = std::env::temp_dir().join("crew_pane_title_dir");
    std::fs::create_dir_all(&base).unwrap();
    let grid = GridSize { cols: 40, rows: 10 };
    let mut pane = spawn_pane("sh", "sh", grid, Some(&base)).unwrap();
    // The open directory's folder name is the title…
    assert_eq!(pane.title_text(), "crew_pane_title_dir");
    // …but an explicit /name still wins.
    pane.name = Some("build".into());
    assert_eq!(pane.title_text(), "build");
}

#[test]
fn terminal_title_appends_the_foreground_command() {
    let base = std::env::temp_dir().join("crew_pane_title_cmd");
    std::fs::create_dir_all(&base).unwrap();
    let grid = GridSize { cols: 40, rows: 10 };
    let mut pane = spawn_pane("sh", "sh", grid, Some(&base)).unwrap();
    // Idle shell → just the folder name.
    assert_eq!(pane.title_text(), "crew_pane_title_cmd");
    // A running command rides alongside the directory.
    if let PaneContent::Terminal(t) = &mut pane.content {
        t.cmd = Some("claude".into());
    }
    assert_eq!(pane.title_text(), "crew_pane_title_cmd · claude");
    // A /name override still wins outright.
    pane.name = Some("build".into());
    assert_eq!(pane.title_text(), "build");
}

#[test]
fn title_text_prefers_user_name() {
    let mut p = Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Settings(SettingsPane::new(CrewConfig::default(), vec![])),
        grid: GridSize { cols: 80, rows: 24 },
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: crate::anim::now_ms(),
        zoom: None,
        tab: 0,
    };
    assert_eq!(p.title_text(), "settings");
    p.name = Some("build".into());
    assert_eq!(p.title_text(), "build");
}

#[test]
fn a_shell_that_will_not_start_names_both_tries_once() {
    let grid = GridSize { cols: 40, rows: 10 };
    let Err(e) = spawn_pane("/nonexistent/zsh", "/nonexistent/sh", grid, None) else {
        panic!("neither shell exists");
    };
    // The status wraps it as `could not open a shell (…)`: no second
    // "failed to spawn shell" inside, and the cause after the tries.
    let shown = format!("{e:#}");
    assert!(
        shown.starts_with("tried /nonexistent/zsh, then /nonexistent/sh: "),
        "{shown}"
    );
}

/// A program's title stack never reaches the name the user gave: a pop sets
/// the OSC title back, and `title_text` reads the name first.
#[test]
fn a_user_name_wins_over_a_popped_title() {
    use crew_term::TermModel;
    let grid = GridSize { cols: 40, rows: 10 };
    let mut pane = spawn_pane("sh", "sh", grid, None).unwrap();
    pane.dir = None;
    let PaneContent::Terminal(t) = &mut pane.content else {
        panic!("a shell pane");
    };
    t.cmd = None;
    t.pty
        .feed(b"\x1b]0;shell\x07\x1b[22;0t\x1b]0;vim\x07\x1b[23;0t");
    assert_eq!(
        pane.title_text(),
        "shell",
        "the pop brought the shell's back"
    );
    pane.name = Some("build".into());
    let PaneContent::Terminal(t) = &mut pane.content else {
        unreachable!()
    };
    t.pty.feed(b"\x1b[22;0t\x1b]0;vim\x07\x1b[23;0t");
    assert_eq!(pane.title_text(), "build");
}
