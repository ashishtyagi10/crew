//! A window cannot be built without an event loop, so what is tested here is
//! everything around one: the queue that defers creation to the callback that
//! can, and the routing that keeps a document window's events out of the grid.
use crate::app::CrewApp;
use crate::viewpane::keys::{view_key, ViewInput};
use winit::keyboard::{Key, SmolStr};

fn tmp_file(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(name);
    std::fs::write(&p, "# a document\n\nwith a line in it.\n").expect("write");
    p
}

/// A window can only be created from a winit callback holding the ACTIVE
/// event loop. Every other path has to queue, or it would have to invent one.
#[test]
fn asking_for_a_window_queues_it_rather_than_failing() {
    let mut app = CrewApp::default();
    let path = tmp_file("docwin-queue.md");
    app.queue_doc_window(&path.to_string_lossy());
    assert_eq!(app.pending_docs, vec![path], "queued for the next tick");
}

/// …and a typo answers where it was typed, not silently one tick later.
#[test]
fn a_path_that_is_not_a_file_says_so_and_queues_nothing() {
    let mut app = CrewApp::default();
    app.queue_doc_window("/definitely/not/here.md");
    assert!(app.pending_docs.is_empty());
    let said = app
        .status
        .as_ref()
        .map(|(s, _)| s.clone())
        .unwrap_or_default();
    assert!(said.contains("not a file"), "said: {said:?}");
}

/// With no document windows open, no event can belong to one — the grid's
/// handler must keep seeing everything it saw before this existed.
#[test]
fn nothing_is_a_document_window_until_one_is_open() {
    let app = CrewApp::default();
    assert!(app.docs.is_empty());
}

/// `w` is the key that takes a document out of the grid. It has to be a key
/// the viewer did not already spend on something else.
#[test]
fn w_is_the_pop_out_key_and_the_others_are_untouched() {
    let k = |c: &str| view_key(&Key::Character(SmolStr::new(c)), true, false);
    assert_eq!(k("w"), ViewInput::PopOut);
    assert_eq!(k("W"), ViewInput::PopOut, "case-folded, like e/o/r/s/v");
    assert_eq!(k("e"), ViewInput::Edit);
    assert_eq!(k("o"), ViewInput::OpenExternal);
    assert_eq!(k("r"), ViewInput::Reload);
    assert_eq!(k("s"), ViewInput::ToggleRaw);
    assert_eq!(k("v"), ViewInput::ToggleSplit);
}
