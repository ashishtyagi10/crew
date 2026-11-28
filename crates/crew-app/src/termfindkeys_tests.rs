use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent, TermPane};
use crate::termfind::FindKey;
use crew_term::{GridSize, PtyTerm, TermModel};
use std::io::Write;
use std::sync::{Arc, Mutex};

/// A writer that remembers what the program in the pane was sent.
struct Tap(Arc<Mutex<Vec<u8>>>);

impl Write for Tap {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// An app with one terminal pane holding `text`, and the tap on its input.
fn app_with(text: &str) -> (CrewApp, Arc<Mutex<Vec<u8>>>) {
    let grid = GridSize { cols: 30, rows: 6 };
    let tmp = std::env::temp_dir();
    let mut pty = PtyTerm::spawn_in(grid, "/bin/sh", &[], Some(&tmp)).unwrap();
    pty.feed(text.as_bytes());
    let sent = Arc::new(Mutex::new(Vec::new()));
    let mut app = CrewApp::default();
    app.panes.push(Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Terminal(Box::new(TermPane {
            pty,
            input: Box::new(Tap(sent.clone())),
            cmd: None,
            cmd_since: None,
            tail: Default::default(),
            read_at: 0,
            spans: Default::default(),
            trail: Default::default(),
            find: None,
            images: Default::default(),
        })),
        grid,
        rect: crate::spawn::PLACEHOLDER_RECT,
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
    });
    app.focused = 0;
    (app, sent)
}

fn term(app: &CrewApp) -> &TermPane {
    match &app.panes[0].content {
        PaneContent::Terminal(t) => t,
        _ => unreachable!(),
    }
}

fn type_str(app: &mut CrewApp, s: &str) {
    for c in s.chars() {
        assert!(app.term_find_input(FindKey::Char(c)));
    }
}

fn lines() -> String {
    (0..40).map(|i| format!("line {i} needle\r\n")).collect()
}

#[cfg(unix)]
#[test]
fn typing_in_the_bar_filters_live_and_stays_out_of_the_program() {
    let (mut app, sent) = app_with(&lines());
    assert!(
        !app.term_find_input(FindKey::Char('a')),
        "closed: the pane's"
    );
    assert!(app.term_find_open());
    type_str(&mut app, "nee");
    assert_eq!(term(&app).find.as_ref().unwrap().matches.len(), 40);
    type_str(&mut app, "dlx");
    assert_eq!(
        term(&app).find.as_ref().unwrap().count().unwrap().0,
        "no matches"
    );
    assert!(app.term_find_input(FindKey::Backspace));
    assert!(app.term_find_input(FindKey::Backspace));
    assert_eq!(term(&app).find.as_ref().unwrap().query, "need");
    assert!(app.term_find_input(FindKey::Other), "stray keys are eaten");
    assert!(sent.lock().unwrap().is_empty(), "nothing reached the PTY");
}

#[cfg(unix)]
#[test]
fn enter_steps_back_the_view_follows_and_esc_gives_the_keys_back() {
    let (mut app, sent) = app_with(&lines());
    assert!(app.term_find_open());
    type_str(&mut app, "line 3 ");
    // "line 3 needle" only: one match, on screen? it is 36 lines back.
    let f = term(&app).find.as_ref().unwrap();
    assert_eq!(f.matches.len(), 1);
    let m = f.matches[0];
    let t = term(&app);
    let row = i64::from(m.line) + t.pty.display_offset() as i64;
    assert!(
        (1..6).contains(&row),
        "the view scrolled to the match: {row}"
    );
    assert!(app.term_find_input(FindKey::Older));
    assert!(app.term_find_input(FindKey::Newer));
    // Close: the bar goes, the view stays where the match put it.
    let off = term(&app).pty.display_offset();
    assert!(off > 0);
    assert!(app.term_find_input(FindKey::Close));
    assert!(term(&app).find.is_none());
    assert_eq!(
        term(&app).pty.display_offset(),
        off,
        "close leaves the view"
    );
    assert!(
        !app.term_find_input(FindKey::Char('a')),
        "keys are the pane's again"
    );
    assert!(sent.lock().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn the_bar_yields_the_keys_to_the_input_bar() {
    let (mut app, _) = app_with("hello\r\n");
    assert!(app.term_find_open());
    app.input.focused = true;
    assert!(!app.term_find_input(FindKey::Char('a')));
}

#[cfg(unix)]
#[test]
fn real_keys_are_the_bars_while_open_and_chords_still_pass() {
    use winit::keyboard::{Key, ModifiersState as M, NamedKey};
    let ch = |s: &str| Key::Character(s.into());
    let (mut app, sent) = app_with(&lines());
    let none = M::empty();
    assert!(
        !app.term_find_event(&ch("n"), true, none),
        "closed: the pane's"
    );
    assert!(app.term_find_open());
    assert!(app.term_find_event(&ch("n"), true, none));
    assert!(
        app.term_find_event(&ch("n"), false, none),
        "a release is eaten too"
    );
    assert!(app.term_find_event(&Key::Named(NamedKey::Space), true, none));
    assert!(
        app.term_find_event(&ch("c"), true, M::CONTROL),
        "Ctrl+C is not the program's"
    );
    assert!(app.term_find_event(&Key::Named(NamedKey::ArrowLeft), true, none));
    assert_eq!(term(&app).find.as_ref().unwrap().query, "n ");
    assert!(app.term_find_event(&Key::Named(NamedKey::Enter), true, M::SHIFT));
    let cmd = if cfg!(target_os = "macos") {
        M::SUPER
    } else {
        M::ALT
    };
    assert!(
        !app.term_find_event(&ch("k"), true, cmd),
        "a crew chord passes"
    );
    assert!(app.term_find_event(&Key::Named(NamedKey::Escape), true, none));
    assert!(
        !app.term_find_event(&ch("n"), true, none),
        "Esc gave the keys back"
    );
    assert!(sent.lock().unwrap().is_empty(), "nothing reached the PTY");
}

#[cfg(unix)]
#[test]
fn composed_text_goes_to_the_query_not_the_program() {
    let (mut app, sent) = app_with("a line\r\n");
    assert!(!app.term_find_text("x"), "no bar: the program's");
    assert!(app.term_find_open());
    assert!(app.term_find_text("\u{65e5}\u{672c}"));
    assert_eq!(term(&app).find.as_ref().unwrap().query, "\u{65e5}\u{672c}");
    assert!(sent.lock().unwrap().is_empty());
}

#[path = "termfindpass_tests.rs"]
mod pass;
