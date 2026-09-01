//! The claim the whole editor is built on, checked against a document nobody wrote for the
//! purpose: **a save touches only what was edited.**
//!
//! `docs/CREW.md` is the repo's own manual — thousands of lines of tables, code fences, nested
//! lists, links and unicode — and it is used here precisely because no fixture would have half
//! its corners. A serializing editor cannot pass this: re-emitting a parsed tree renormalizes
//! bullets, re-wraps paragraphs and rewrites setext headings, and the diff comes back hundreds
//! of lines long. Here the buffer IS the source, so the diff can only be what was typed.
use super::*;
use crate::viewpane::detect::Format;
use crate::viewpane::load::Loaded;
use crate::viewpane::LoadState;

const COLS: u16 = 92;
const ROWS: u16 = 40;

/// The manual, as it is on disk.
fn manual() -> (std::path::PathBuf, String) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/CREW.md")
        .canonicalize()
        .expect("the repo's own manual");
    let text = std::fs::read_to_string(&path).expect("readable");
    (path, text)
}

/// A pane holding `text`, saving to a scratch file of its own — the test edits the manual and
/// never writes to it.
fn pane(text: &str, tag: &str) -> ViewPane {
    let out = std::env::temp_dir().join(format!("crew-savediff-{}-{tag}.md", std::process::id()));
    let _ = std::fs::remove_file(&out);
    let mut p = ViewPane::open(out);
    p.state = LoadState::Ready {
        format: Format::Markdown,
        loaded: Loaded {
            text: text.into(),
            truncated: None,
            meta: None,
            image: None,
        },
    };
    p.start_editing(COLS);
    p
}

fn saved(p: &mut ViewPane) -> String {
    p.save().expect("saved");
    std::fs::read_to_string(&p.path).expect("readable")
}

/// The bytes that differ, as (offset, what was there, what is there now).
fn one_difference(before: &str, after: &str) -> (usize, String, String) {
    let (b, a) = (before.as_bytes(), after.as_bytes());
    let head = b.iter().zip(a).take_while(|(x, y)| x == y).count();
    let tail = b[head..]
        .iter()
        .rev()
        .zip(a[head..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    (
        head,
        String::from_utf8_lossy(&b[head..b.len() - tail]).into_owned(),
        String::from_utf8_lossy(&a[head..a.len() - tail]).into_owned(),
    )
}

#[test]
fn opening_the_manual_and_saving_it_untouched_writes_it_back_byte_for_byte() {
    // No edit at all: the strongest form of the claim, and the one a serializer fails first.
    let (_, text) = manual();
    let mut p = pane(&text, "untouched");
    assert_eq!(saved(&mut p), text);
}

#[test]
fn typing_one_word_into_the_manual_changes_one_word_of_the_file() {
    let (_, text) = manual();
    let mut p = pane(&text, "typed");
    // Somewhere deep in the document, past every table and fence above it.
    let at = text.rfind("crew").expect("the word crew is in the manual");
    p.after_edit(at, COLS, ROWS);
    p.insert("XYZZY", COLS, ROWS);
    let out = saved(&mut p);
    let (offset, was, now) = one_difference(&text, &out);
    assert_eq!(offset, at, "the change is where the caret was");
    assert_eq!(was, "", "nothing was removed");
    assert_eq!(now, "XYZZY", "and only what was typed was added");
    assert_eq!(
        out.len(),
        text.len() + 5,
        "the rest of the file is the rest"
    );
}

#[test]
fn deleting_inside_a_table_leaves_every_other_row_alone() {
    // Tables are where a re-serializing editor does its worst work: column widths get
    // recomputed and every row of the table changes.
    let (_, text) = manual();
    let row = text.find("\n| ").expect("the manual has a table");
    let mut p = pane(&text, "table");
    p.after_edit(row + 3, COLS, ROWS);
    p.delete(COLS, ROWS);
    let out = saved(&mut p);
    let (offset, was, now) = one_difference(&text, &out);
    assert_eq!(offset, row + 3);
    assert_eq!(
        was.chars().count(),
        1,
        "exactly one character left: {was:?}"
    );
    assert_eq!(now, "", "and nothing took its place");
}

#[test]
fn an_edit_then_an_undo_writes_the_file_it_started_as() {
    let (_, text) = manual();
    let mut p = pane(&text, "undone");
    let at = text.find("\n## ").expect("a heading");
    p.after_edit(at + 4, COLS, ROWS);
    p.insert("Not really ", COLS, ROWS);
    assert!(p.undo(COLS, ROWS));
    assert_eq!(saved(&mut p), text);
}
