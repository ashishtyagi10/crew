//! The app's end of hint mode: opening it, and what the keys do to it after.
use super::*;
use crate::hints::Hints;
use crate::pane::{Pane, PaneContent};
use crate::viewpane::{LoadState, ViewPane};

/// The tests share one process-wide mode (as the app does), so each starts
/// from a known one.
fn app_with_view(text: &str) -> CrewApp {
    crate::hints::close();
    let mut app = CrewApp::default();
    let mut v = ViewPane::open(std::env::temp_dir().join("hints.txt"));
    v.state = LoadState::Ready {
        format: crate::viewpane::detect::Format::Code { lang: "" },
        loaded: crate::viewpane::load::Loaded {
            text: text.into(),
            truncated: None,
            meta: None,
            image: None,
        },
    };
    app.panes.push(Pane {
        glide: Default::default(),
        content: PaneContent::View(v),
        grid: crew_term::GridSize { cols: 60, rows: 8 },
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
    });
    app
}

fn key(text: &str) -> HintKey {
    HintKey::Typed(text.chars().next().expect("a character"))
}

fn esc() -> HintKey {
    HintKey::Escape
}

#[test]
fn the_chord_labels_the_focused_pane() {
    let mut app = app_with_view("see docs/CREW.md and https://example.invalid/x\n");
    app.open_hints();
    assert!(crate::hints::active(), "the mode opened");
}

/// A pane with nothing to reach must not open a mode — it would swallow the
/// next key you pressed and give nothing back for it.
#[test]
fn a_pane_with_nothing_to_reach_opens_no_mode() {
    let mut app = app_with_view("all quiet here\nnothing to see\n");
    app.open_hints();
    assert!(!crate::hints::active());
    crate::hints::close();
}

/// While the mode is on it owns the keyboard — that is what makes a single
/// letter mean "that one".
#[test]
fn every_key_is_consumed_while_the_mode_is_on() {
    let mut app = app_with_view("docs/CREW.md\n");
    assert!(!app.hint_input(key("a")), "off, keys belong to the pane");
    app.open_hints();
    assert!(app.hint_input(key("z")), "on, the mode takes them");
    crate::hints::close();
}

#[test]
fn escape_ends_the_mode_and_the_keys_go_back_to_the_pane() {
    let mut app = app_with_view("docs/CREW.md\n");
    app.open_hints();
    assert!(app.hint_input(esc()));
    assert!(!crate::hints::active());
    assert!(!app.hint_input(key("a")), "keys are the pane's again");
}

/// A letter that starts no label ends the mode rather than sitting there
/// eating keys.
#[test]
fn a_miss_ends_the_mode() {
    let mut app = app_with_view("docs/CREW.md\n");
    app.open_hints();
    let label = crate::hints::labels_snapshot()
        .first()
        .cloned()
        .expect("a label");
    let other = ('a'..='z')
        .find(|c| !label.starts_with(*c))
        .expect("some other letter");
    app.hint_input(key(&other.to_string()));
    assert!(!crate::hints::active(), "a miss must not leave it open");
}

/// The mode is a property of ONE pane's contents. Picking ends it, so the
/// labels can never be left over a pane that has since scrolled.
#[test]
fn picking_a_label_ends_the_mode() {
    let mut app = app_with_view("docs/CREW.md\n");
    app.open_hints();
    let label = crate::hints::labels_snapshot()
        .first()
        .cloned()
        .expect("a label");
    app.hint_input(key(&label));
    assert!(!crate::hints::active());
}

/// Scanning is over the pane's rendered rows, so what is labelled is exactly
/// what is on screen — not what is in the file above or below it.
#[test]
fn only_what_the_pane_is_showing_gets_a_label() {
    let mut lines = String::new();
    for i in 0..40 {
        lines.push_str(&format!("file{i}.txt\n"));
    }
    let mut app = app_with_view(&lines);
    app.open_hints();
    let n = crate::hints::labels_snapshot().len();
    crate::hints::close();
    assert!(n > 0 && n <= 8, "labelled {n} of 40 lines on an 8-row pane");
}

#[test]
fn a_scan_of_rows_finds_nothing_in_an_empty_pane() {
    assert!(Hints::scan(&[]).is_none());
}
