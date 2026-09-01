//! The field, over a real document. The window is not needed for any of it — which is the point
//! of keeping the logic on the pane — so these run headless and check the only thing that
//! matters in the end: what the file says afterwards.
use super::*;
use crate::viewpane::detect::Format;
use crate::viewpane::load::Loaded;
use crate::viewpane::LoadState;

const DOC: &str = "See [the docs](https://old.example.com) for more.\n";

const COLS: u16 = 60;
const ROWS: u16 = 20;

fn doc(text: &str) -> ViewPane {
    let mut p = ViewPane::open(std::env::temp_dir().join("link.md"));
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

/// Put the caret on the first byte of `needle`.
fn caret_on(p: &mut ViewPane, needle: &str) {
    let at = p.source_str().unwrap().find(needle).expect("needle") as u32;
    p.after_edit(at as usize, COLS, ROWS);
}

fn src(p: &ViewPane) -> String {
    p.source_str().unwrap_or_default().to_string()
}

#[test]
fn the_field_opens_holding_the_url_that_is_already_there() {
    let mut p = doc(DOC);
    caret_on(&mut p, "the docs");
    let f = UrlEdit::open(&mut p, COLS, ROWS).expect("a link under the caret");
    assert_eq!(f.buf, "https://old.example.com");
    assert_eq!(f.undos, 0, "nothing was written to open it");
    assert_eq!(src(&p), DOC, "and the document is untouched");
}

#[test]
fn typing_a_url_and_pressing_enter_replaces_only_the_url() {
    let mut p = doc(DOC);
    caret_on(&mut p, "the docs");
    let mut f = UrlEdit::open(&mut p, COLS, ROWS).unwrap();
    f.buf.clear();
    for c in "https://new.example.com".chars() {
        assert_eq!(f.take(FieldKey::Type(c.to_string())), None);
    }
    assert_eq!(f.take(FieldKey::Apply), Some(true));
    f.apply(&mut p, COLS, ROWS);
    assert_eq!(
        src(&p),
        "See [the docs](https://new.example.com) for more.\n"
    );
}

#[test]
fn escape_leaves_the_document_exactly_as_it_was() {
    let mut p = doc(DOC);
    caret_on(&mut p, "the docs");
    let mut f = UrlEdit::open(&mut p, COLS, ROWS).unwrap();
    f.take(FieldKey::Type("nonsense".into()));
    assert_eq!(f.take(FieldKey::Cancel), Some(false));
    f.cancel(&mut p, COLS, ROWS);
    assert_eq!(src(&p), DOC);
}

#[test]
fn backspace_takes_back_one_character_of_the_url() {
    let mut f = UrlEdit {
        from: 0,
        to: 0,
        buf: "abc".into(),
        undos: 0,
    };
    assert_eq!(f.take(FieldKey::Backspace), None);
    assert_eq!(f.buf, "ab");
}

#[test]
fn a_selection_becomes_a_link_with_the_field_open_on_its_empty_url() {
    let mut p = doc("Read the manual today.\n");
    caret_on(&mut p, "the manual");
    p.anchor_here();
    for _ in 0..10 {
        p.move_caret(crate::viewpane::caret::Step::Right, COLS, ROWS);
    }
    let mut f = UrlEdit::open(&mut p, COLS, ROWS).expect("a selection to link");
    assert_eq!(
        f.undos, 2,
        "a scaffold: the selection deleted, the link written"
    );
    assert_eq!(f.buf, "", "with an empty URL to type into");
    assert_eq!(src(&p), "Read [the manual]() today.\n");
    f.buf.push_str("http://a");
    f.apply(&mut p, COLS, ROWS);
    assert_eq!(src(&p), "Read [the manual](http://a) today.\n");
}

#[test]
fn cancelling_a_link_that_was_just_made_takes_the_scaffold_back_out() {
    // The failure this exists for: `[half a thought]()` left in the file because somebody
    // changed their mind, which is worse than having no shortcut at all.
    let mut p = doc("Read the manual today.\n");
    caret_on(&mut p, "the manual");
    p.anchor_here();
    for _ in 0..10 {
        p.move_caret(crate::viewpane::caret::Step::Right, COLS, ROWS);
    }
    let f = UrlEdit::open(&mut p, COLS, ROWS).unwrap();
    f.cancel(&mut p, COLS, ROWS);
    assert_eq!(src(&p), "Read the manual today.\n");
}

#[test]
fn with_neither_a_link_nor_a_selection_the_chord_says_what_to_do() {
    let mut p = doc("Just a paragraph.\n");
    caret_on(&mut p, "paragraph");
    let why = UrlEdit::open(&mut p, COLS, ROWS).unwrap_err();
    assert!(why.contains("select some words"), "{why}");
    assert_eq!(src(&p), "Just a paragraph.\n", "and writes nothing");
}

#[test]
fn the_field_answers_the_keys_a_field_answers_and_no_others() {
    use winit::keyboard::{Key, NamedKey};
    assert_eq!(
        field_key(&Key::Named(NamedKey::Enter), true),
        Some(FieldKey::Apply)
    );
    assert_eq!(
        field_key(&Key::Named(NamedKey::Escape), true),
        Some(FieldKey::Cancel)
    );
    assert_eq!(
        field_key(&Key::Character("x".into()), true),
        Some(FieldKey::Type("x".into()))
    );
    // A key going UP is not a keystroke, and an arrow is the window's, not the field's.
    assert_eq!(field_key(&Key::Character("x".into()), false), None);
    assert_eq!(field_key(&Key::Named(NamedKey::ArrowLeft), true), None);
}
