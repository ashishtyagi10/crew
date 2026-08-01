use std::path::{Path, PathBuf};

use crew_term::GridSize;

use super::{mention_token, shell_quoted};
use crate::app::CrewApp;
use crate::layout::Rect;
use crate::pane::{Pane, PaneContent};

#[test]
fn a_path_inside_the_cwd_mentions_relative() {
    let tok = mention_token(Path::new("/proj/src/main.rs"), Path::new("/proj"));
    assert_eq!(tok, "@src/main.rs "); // trailing space ends the token
}

#[test]
fn a_path_outside_the_cwd_mentions_absolute() {
    let tok = mention_token(Path::new("/etc/hosts"), Path::new("/proj"));
    assert_eq!(tok, "@/etc/hosts ");
}

#[test]
fn dropping_the_cwd_itself_stays_absolute() {
    // A bare strip_prefix would leave "@ " — a token that mentions nothing.
    let tok = mention_token(Path::new("/proj"), Path::new("/proj"));
    assert_eq!(tok, "@/proj ");
}

#[test]
fn a_path_with_spaces_mentions_quoted() {
    // `chatmention::expand` tokenizes on whitespace: a bare token containing
    // a space would silently die at send. The quoted form survives it.
    let tok = mention_token(Path::new("/proj/My Documents/x.pdf"), Path::new("/proj"));
    assert_eq!(tok, "@\"My Documents/x.pdf\" ");
    let tok = mention_token(Path::new("/etc/odd dir/f.txt"), Path::new("/proj"));
    assert_eq!(tok, "@\"/etc/odd dir/f.txt\" ");
}

#[test]
fn shell_quoting_wraps_and_escapes_like_a_terminal_paste() {
    assert_eq!(shell_quoted(Path::new("/a/plain.txt")), "'/a/plain.txt' ");
    assert_eq!(
        shell_quoted(Path::new("/a/my file.txt")),
        "'/a/my file.txt' "
    );
    assert_eq!(shell_quoted(Path::new("/a/it's.txt")), r"'/a/it'\''s.txt' ");
}

/// An app whose focused pane is a chat pane opened over `dir` (None = the
/// app cwd `/proj`), mirroring `chatspawn`'s pane construction.
fn chat_app(dir: Option<&str>) -> CrewApp {
    let mut app = CrewApp {
        cwd: PathBuf::from("/proj"),
        ..Default::default()
    };
    app.panes.push(Pane {
        content: PaneContent::Chat(crate::chat::tests::pane()),
        grid: GridSize { cols: 80, rows: 24 },
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: None,
        dir: dir.map(PathBuf::from),
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: 0,
    });
    app
}

fn chat_input(app: &CrewApp) -> &str {
    match &app.panes[0].content {
        PaneContent::Chat(c) => &c.input,
        _ => unreachable!(),
    }
}

#[test]
fn drops_append_mention_tokens_to_the_chat_composer() {
    let mut app = chat_app(None);
    app.drop_file(Path::new("/proj/src/main.rs"));
    app.drop_file(Path::new("/etc/hosts"));
    // Second drop appends after the first's trailing space — never replaces.
    assert_eq!(chat_input(&app), "@src/main.rs @/etc/hosts ");
}

#[test]
fn a_drop_closes_a_mention_popup_left_open_mid_typing() {
    let mut app = chat_app(None);
    if let PaneContent::Chat(c) = &mut app.panes[0].content {
        c.input = "see @sr".into(); // a mention mid-typing, popup open
        c.mention = Some(crate::chatmention::MentionState {
            entries: Vec::new(),
            matches: Vec::new(),
            sel: 0,
        });
    }
    app.drop_file(Path::new("/proj/src/main.rs"));
    // The completed token (trailing space) is no pending mention: the popup
    // closes exactly as after a typed edit — no stale matches linger.
    if let PaneContent::Chat(c) = &app.panes[0].content {
        assert!(c.mention.is_none());
    }
}

#[test]
fn a_drop_after_text_inserts_a_separating_space() {
    // Without it the token glues onto the last word ("summarize@src/main.rs")
    // and `chatmention::expand` never sees a mention at send.
    let mut app = chat_app(None);
    if let PaneContent::Chat(c) = &mut app.panes[0].content {
        c.input = "summarize".into();
    }
    app.drop_file(Path::new("/proj/src/main.rs"));
    assert_eq!(chat_input(&app), "summarize @src/main.rs ");
}

#[test]
fn a_drop_mid_mention_starts_a_fresh_token() {
    // Mid-typed "@sr" must not become "@sr@src/main.rs" — one broken token.
    let mut app = chat_app(None);
    if let PaneContent::Chat(c) = &mut app.panes[0].content {
        c.input = "see @sr".into();
    }
    app.drop_file(Path::new("/proj/src/main.rs"));
    assert_eq!(chat_input(&app), "see @sr @src/main.rs ");
}

#[test]
fn mentions_relativize_against_the_send_time_cwd_not_the_panes_dir() {
    // Send-time `chatmention::expand` resolves against `self.cwd` (keys.rs
    // hands it exactly that) — the drop-time token must use the SAME root,
    // or a token minted relative to `pane.dir` dies silently at send.
    let mut app = chat_app(Some("/other"));
    app.drop_file(Path::new("/other/notes.md"));
    assert_eq!(chat_input(&app), "@/other/notes.md ");
    app.drop_file(Path::new("/proj/x.rs"));
    assert_eq!(chat_input(&app), "@/other/notes.md @x.rs ");
}

#[test]
fn a_drop_while_the_search_popup_is_open_closes_it_and_keeps_the_draft() {
    // Ctrl+R's Close/Accept restore the composer from its `saved` snapshot —
    // a token appended while it is open would be silently thrown away.
    let mut app = chat_app(None);
    if let PaneContent::Chat(c) = &mut app.panes[0].content {
        c.input = "my draft".into();
        c.history.record("earlier");
        c.on_input(crate::chatkeys::ChatInput::HistSearch, Path::new("/proj"));
        assert!(c.histsearch.is_some(), "premise: the search popup is open");
    }
    app.drop_file(Path::new("/proj/src/main.rs"));
    if let PaneContent::Chat(c) = &app.panes[0].content {
        assert!(c.histsearch.is_none(), "the drop closed the search first");
        assert_eq!(
            c.input, "my draft @src/main.rs ",
            "prior draft intact, token appended"
        );
    }
}

#[test]
fn a_drop_on_a_terminal_pane_writes_the_quoted_path() {
    let mut app = CrewApp::default();
    let grid = GridSize { cols: 40, rows: 10 };
    app.panes
        .push(crate::pane::spawn_pane("sh", "sh", grid, None).unwrap());
    app.drop_file(Path::new("/a/my file.txt"));
    // The PTY write itself has no readable side to assert without racing the
    // shell; the status note carries the exact bytes-minus-trailing-space.
    assert_eq!(
        app.active_status(),
        Some("dropped file \u{2192} '/a/my file.txt'")
    );
}

#[test]
fn hidden_panes_and_empty_apps_ignore_drops() {
    let mut app = chat_app(None);
    app.panes[0].hidden = true;
    app.drop_file(Path::new("/proj/a.txt"));
    assert_eq!(
        chat_input(&app),
        "",
        "a minimized pane must not receive text"
    );

    let mut empty = CrewApp::default();
    empty.drop_file(Path::new("/proj/a.txt")); // no panes: silently ignored
    assert!(empty.active_status().is_none());
}
