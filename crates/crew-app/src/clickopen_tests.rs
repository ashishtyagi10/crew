use super::{token_at, view_link_at};
use crate::app::CrewApp;
use crate::chatbody::{plain, CardCell, CardLine};
use crate::pane::PaneContent;

/// A one-line fixture whose whole line is a link (mirrors how
/// `mdrung::lines`/`chatmd` tag a link span's cells).
fn link_line(text: &str, url: &str) -> CardLine {
    text.chars()
        .map(|c| CardCell {
            link: Some(std::sync::Arc::from(url)),
            src: None,
            ..plain(c, (0, 0, 0), false)
        })
        .collect()
}

#[test]
fn view_link_at_hits_a_link_cell_and_misses_plain_text() {
    let lines = vec![
        vec![plain('p', (0, 0, 0), false), plain('l', (0, 0, 0), false)],
        link_line("go", "https://x.io"),
    ];
    assert_eq!(
        view_link_at(&lines, 0, 1, 0),
        Some("https://x.io".to_string()),
        "the link line resolves"
    );
    assert_eq!(
        view_link_at(&lines, 0, 0, 0),
        None,
        "plain text is never a link"
    );
}

#[test]
fn view_link_at_respects_the_scroll_offset() {
    // Three lines, scrolled down by 1: pane-relative row 0 is source
    // line 1 (the link), not line 0.
    let lines = vec![
        vec![plain('a', (0, 0, 0), false)],
        link_line("b", "https://s.io"),
        vec![plain('c', (0, 0, 0), false)],
    ];
    assert_eq!(
        view_link_at(&lines, 1, 0, 0),
        Some("https://s.io".to_string())
    );
    // Without the scroll, row 0 is the plain first line — no link.
    assert_eq!(view_link_at(&lines, 0, 0, 0), None);
}

#[test]
fn view_link_at_clamps_a_stale_scroll_to_match_what_cells_draws() {
    // If `scroll` is momentarily stale (larger than the content — e.g.
    // right after a reload shrank the file, before `clamp_scroll` has
    // run again), `ViewPane::cells` still draws SOMETHING at row 0: the
    // last line, per its own `top = scroll.min(len - 1)` clamp
    // (`viewpane::render`). This must resolve the same line, or a click
    // could silently miss content that is visibly on screen.
    let lines = vec![
        vec![plain('a', (0, 0, 0), false)],
        link_line("b", "https://last.io"),
    ];
    assert_eq!(
        view_link_at(&lines, 999, 0, 0),
        Some("https://last.io".to_string()),
        "an over-large scroll still resolves the line actually drawn at row 0"
    );
}

#[test]
fn view_link_at_out_of_bounds_is_none_not_a_panic() {
    let lines = vec![link_line("x", "https://x.io")];
    assert_eq!(view_link_at(&lines, 0, 50, 0), None, "row past the end");
    assert_eq!(view_link_at(&lines, 0, 0, 50), None, "col past the end");
    assert_eq!(view_link_at(&[], 0, 0, 0), None, "no lines at all");
}

#[test]
fn token_at_extracts_word_and_trims_punctuation() {
    let line = "edit src/main.rs, please";
    let i = line.find("src").unwrap();
    assert_eq!(token_at(line, i + 1).as_deref(), Some("src/main.rs"));
    // surrounding quotes/parens are stripped.
    assert_eq!(token_at("(foo/bar)", 2).as_deref(), Some("foo/bar"));
    assert_eq!(token_at("\"a/b\"", 2).as_deref(), Some("a/b"));
}

#[test]
fn token_at_over_whitespace_is_none() {
    assert_eq!(token_at("a b", 1), None);
    assert_eq!(token_at("word", 99), None);
    // a token that is only punctuation trims to nothing.
    assert_eq!(token_at("(),", 0), None);
}

#[test]
fn a_path_an_agent_wrote_opens_the_viewer() {
    // The file the agent just changed should be one click away, without
    // leaving the transcript.
    let dir = std::env::temp_dir();
    let f = dir.join("agent-cited.rs");
    std::fs::write(&f, "fn main() {}\n").unwrap();

    let mut chat = crate::chat::tests::pane();
    chat.push_capped(crate::chatlayout::Message {
        sender: "agent smith".into(),
        text: "see agent-cited.rs for the fix".into(),
        ts: "1".into(),
        meta: String::new(),
        usage: None,
        expanded: false,
    });
    let (cols, rows) = (80u16, 20u16);
    // Locate where the path actually rendered rather than hardcoding
    // layout — mirrors `chatview_tests`' own click-target lookups.
    let mut rendered: std::collections::BTreeMap<u16, String> = Default::default();
    for c in crate::chatview::cells(&chat, cols, rows) {
        rendered.entry(c.row).or_default().push(c.c);
    }
    let (&row, line) = rendered
        .iter()
        .find(|(_, text)| text.contains("agent-cited.rs"))
        .expect("the path text rendered somewhere");
    let col = line.find("agent-cited.rs").unwrap() as u16;

    let mut app = CrewApp {
        cwd: dir.clone(),
        ..Default::default()
    };
    app.panes.push(crate::pane::Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Chat(chat),
        grid: crew_term::GridSize { cols, rows },
        rect: crate::layout::Rect {
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
    });
    let before = app.panes.len();

    assert!(
        app.resolve_chat_click(0, row, col),
        "clicking the cited path must act"
    );
    assert_eq!(app.panes.len(), before + 1, "a new pane opened");
    assert!(
        app.panes
            .iter()
            .any(|p| matches!(&p.content, PaneContent::View(v) if v.path == f)),
        "the viewer opened on the exact file the agent cited"
    );
}
