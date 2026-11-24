//! OSC 52 reads through a real pane: a shell asks, and what its program is
//! sent is captured in place of its pty input.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]
use super::tests::fake;
use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent, TermPane};
use crew_term::{GridSize, PtyTerm};
use std::sync::{Arc, Mutex};

/// What a pane's program is sent, captured instead of written to its pty.
#[derive(Clone, Default)]
struct Sink(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Sink {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A shell that asks to read the clipboard, then titles itself `asked` so
/// the test knows the ask has been parsed.
fn asking_pane(sink: &Sink) -> Pane {
    let grid = GridSize { cols: 40, rows: 8 };
    let script = "printf '\\033]52;c;?\\007\\033]2;asked\\007'; sleep 5";
    let pty = PtyTerm::spawn_in(grid, "/bin/sh", &["-c".into(), script.into()], None).unwrap();
    Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Terminal(Box::new(TermPane {
            pty,
            input: Box::new(sink.clone()),
            cmd: None,
            cmd_since: None,
            tail: Default::default(),
            read_at: 0,
            spans: Default::default(),
            trail: Default::default(),
            images: Default::default(),
        })),
        grid,
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
        born_ms: 0,
        zoom: None,
        tab: 0,
    }
}

/// Read every pane's output until each has asked.
fn until_asked(app: &mut CrewApp) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    for p in app.panes.iter_mut() {
        let PaneContent::Terminal(t) = &mut p.content else {
            continue;
        };
        while t.pty.title() != "asked" {
            assert!(
                std::time::Instant::now() < deadline,
                "the shell never asked"
            );
            t.pty.try_read();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

/// Two panes ask; with the setting on, only the focused one is answered —
/// and the other's refused ask is gone, not held until it gains focus.
#[cfg(unix)]
#[test]
fn only_the_focused_pane_is_answered_and_a_refusal_is_final() {
    let (bg, fg) = (Sink::default(), Sink::default());
    let mut app = CrewApp::default();
    app.config.clipboard_read = "allow".into();
    app.panes.push(asking_pane(&bg));
    app.panes.push(asking_pane(&fg));
    app.focused = 1;
    until_asked(&mut app);
    let mut reads = 0;
    app.serve_clipboard_asks(&mut fake("hi", &mut reads));
    assert_eq!(*fg.0.lock().unwrap(), b"\x1b]52;c;aGk=\x07");
    assert!(bg.0.lock().unwrap().is_empty(), "a background pane read it");
    assert_eq!(reads, 1);
    app.focused = 0;
    app.serve_clipboard_asks(&mut fake("hi", &mut reads));
    assert!(bg.0.lock().unwrap().is_empty(), "a refused ask was kept");
}

/// The default is deny: the focused pane asks, gets nothing, and the
/// clipboard is never read.
#[cfg(unix)]
#[test]
fn by_default_the_focused_pane_gets_nothing() {
    let sink = Sink::default();
    let mut app = CrewApp::default();
    app.panes.push(asking_pane(&sink));
    until_asked(&mut app);
    let mut reads = 0;
    app.serve_clipboard_asks(&mut fake("hi", &mut reads));
    assert!(sink.0.lock().unwrap().is_empty());
    assert_eq!(reads, 0);
}
