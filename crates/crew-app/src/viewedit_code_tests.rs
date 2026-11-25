//! Editing a file that is not rendered markdown — code, plain text, a markdown file's source
//! view — in a viewer pane, through the same keys a markdown document takes.
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::app::CrewApp;
use crate::pane::PaneContent;
use crate::viewpane::keys::{apply, ViewInput};
use crate::viewpane::{ViewAction, ViewPane};

pub(super) fn app_with(name: &str, text: &str) -> (CrewApp, std::path::PathBuf) {
    app_with_bytes(name, text.as_bytes())
}

pub(super) fn app_with_bytes(name: &str, bytes: &[u8]) -> (CrewApp, std::path::PathBuf) {
    let dir = std::env::temp_dir().join("viewedit-code-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join(name);
    std::fs::write(&f, bytes).unwrap();
    let mut app = CrewApp {
        cwd: dir,
        ..Default::default()
    };
    app.open_view(name);
    for _ in 0..500 {
        let v = view(&mut app);
        v.poll();
        if !v.loading() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    app.focused = 0;
    app.input.focused = false;
    (app, f)
}

pub(super) fn view(app: &mut CrewApp) -> &mut ViewPane {
    match &mut app.panes[0].content {
        PaneContent::View(v) => v,
        _ => panic!("a viewer pane"),
    }
}

/// What `i` (or Enter) answers on this pane.
pub(super) fn ask(app: &mut CrewApp, input: ViewInput) -> Option<ViewAction> {
    let (cols, rows) = (app.panes[0].grid.cols, app.panes[0].grid.rows);
    apply(view(app), input, cols, rows)
}

pub(super) fn typed(app: &mut CrewApp, s: &str) {
    assert!(app.view_edit_input(&Key::Character(s.into()), true, ModifiersState::empty()));
}

pub(super) fn named(app: &mut CrewApp, k: NamedKey) {
    assert!(app.view_edit_input(&Key::Named(k), true, ModifiersState::empty()));
}

pub(super) fn save(app: &mut CrewApp) {
    assert!(app.view_edit_input(&Key::Character("s".into()), true, ModifiersState::SUPER));
}

/// A Rust file: `i` puts the caret on its first character, typing lands there, Cmd+S writes
/// exactly that and nothing else.
#[test]
fn a_code_file_is_typed_into_and_saved() {
    let (mut app, f) = app_with("main.rs", "fn main() {}\n");
    assert!(matches!(
        ask(&mut app, ViewInput::Insert),
        Some(ViewAction::Editing)
    ));
    typed(&mut app, "p");
    save(&mut app);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "pfn main() {}\n");
    assert!(!view(&mut app).dirty);
}

/// Enter in code is a newline that keeps the line's indent — not markdown's blank line.
#[test]
fn enter_in_code_carries_the_indent() {
    let (mut app, f) = app_with("indent.py", "def f():\n    x = 1\n");
    ask(&mut app, ViewInput::Insert);
    named(&mut app, NamedKey::ArrowDown);
    named(&mut app, NamedKey::End);
    named(&mut app, NamedKey::Enter);
    typed(&mut app, "y");
    save(&mut app);
    assert_eq!(
        std::fs::read_to_string(&f).unwrap(),
        "def f():\n    x = 1\n    y\n"
    );
}

/// A blank line is somewhere to stand: Down onto it, type, and the text lands on that line.
#[test]
fn a_blank_line_takes_the_caret() {
    let (mut app, f) = app_with("blank.txt", "one\n\nthree\n");
    ask(&mut app, ViewInput::Insert);
    named(&mut app, NamedKey::ArrowDown);
    typed(&mut app, "two");
    save(&mut app);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "one\ntwo\nthree\n");
}

/// Tabs and CRLF are rewritten for the screen, never in the file: a byte typed after them lands
/// where it was typed.
#[test]
fn tabs_and_crlf_map_back_to_the_file() {
    let (mut app, f) = app_with("tabs.go", "a\r\n\tb\r\n");
    ask(&mut app, ViewInput::Insert);
    named(&mut app, NamedKey::ArrowDown);
    named(&mut app, NamedKey::End);
    typed(&mut app, "c");
    save(&mut app);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "a\r\n\tbc\r\n");
}

/// Enter opens editing too, and the focused frame says it can.
#[test]
fn enter_starts_editing_and_the_legend_says_how() {
    let (mut app, _) = app_with("cue.toml", "a = 1\n");
    assert_eq!(view(&mut app).legend(true), "cue.toml \u{b7} i to edit");
    assert_eq!(view(&mut app).legend(false), "cue.toml");
    assert!(matches!(
        ask(&mut app, ViewInput::Enter),
        Some(ViewAction::Editing)
    ));
    assert_eq!(view(&mut app).legend(true), "cue.toml \u{b7} editing");
}

/// An empty file is still one to write in.
#[test]
fn an_empty_file_can_be_written() {
    let (mut app, f) = app_with("empty.txt", "");
    assert!(matches!(
        ask(&mut app, ViewInput::Insert),
        Some(ViewAction::Editing)
    ));
    typed(&mut app, "hi");
    save(&mut app);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "hi");
}

/// A rendered table is not the file's text; its source view is. A file read lossily is refused
/// outright: saving it would write U+FFFD over every byte that was not UTF-8.
#[test]
fn what_is_not_the_files_text_is_refused_with_a_reason() {
    let (mut app, _) = app_with("t.csv", "a,b\n1,2\n");
    assert!(matches!(
        ask(&mut app, ViewInput::Insert),
        Some(ViewAction::NoEdit(_))
    ));
    ask(&mut app, ViewInput::ToggleRaw);
    assert!(matches!(
        ask(&mut app, ViewInput::Insert),
        Some(ViewAction::Editing)
    ));
    let (mut app, _) = app_with_bytes("latin1.txt", b"caf\xe9\n");
    let act = ask(&mut app, ViewInput::Insert);
    assert!(matches!(act, Some(ViewAction::NoEdit(_))));
}

/// Markdown's source view (`s`) is a text file like any other — Enter there is a plain newline.
#[test]
fn markdown_source_view_edits_as_text() {
    let (mut app, f) = app_with("src.md", "- a\n");
    ask(&mut app, ViewInput::ToggleRaw);
    ask(&mut app, ViewInput::Insert);
    named(&mut app, NamedKey::End);
    named(&mut app, NamedKey::Enter);
    typed(&mut app, "b");
    save(&mut app);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "- a\nb\n");
}
