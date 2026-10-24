use std::path::PathBuf;

use winit::keyboard::NamedKey;

use super::mime_of;
use crate::inbox::compose::On;
use crate::inbox::composekeys::tests::{on, press, reading, typed};
use crate::inbox::worker::Ask;
use crate::inbox::InboxPane;

/// A file of `len` bytes in a folder of its own.
fn file(name: &str, len: u64) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join(name);
    std::fs::File::create(&p).unwrap().set_len(len).unwrap();
    (dir, p)
}

/// The pane as text, a line a row, spaces where nothing is drawn.
fn shown(p: &InboxPane) -> String {
    let mut rows = vec![vec![' '; 80]; 30];
    for c in p.cells(80, 30) {
        if let Some(cell) = rows
            .get_mut(usize::from(c.row))
            .and_then(|r| r.get_mut(usize::from(c.col)))
        {
            *cell = c.c;
        }
    }
    rows.into_iter()
        .map(|r| r.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_file_lands_on_the_row_opens_to_be_seen_and_comes_off() {
    let (mut p, _) = reading();
    let (_d, path) = file("notes.pdf", 12 * 1024);
    typed(&mut p, "r");
    assert!(shown(&p).contains("+ attach a file \u{b7} or drop one on the pane"));
    assert_eq!(p.attach(&path), "attached notes.pdf \u{b7} 12 KB");
    assert_eq!(p.attach(&path), "notes.pdf is attached already");
    assert!(
        shown(&p).contains("notes.pdf 12 KB \u{2715}   + attach"),
        "{}",
        shown(&p)
    );
    press(&mut p, NamedKey::ArrowDown);
    assert_eq!(on(&p), On::Files, "down from the words is the files");
    crate::linkclick::OPENED.with(|o| o.borrow_mut().clear());
    press(&mut p, NamedKey::Enter);
    let opened = crate::linkclick::OPENED.with(|o| o.borrow().clone());
    assert_eq!(
        opened,
        [path.to_string_lossy().to_string()],
        "Enter opens it, to see it"
    );
    typed(&mut p, "x");
    assert_eq!(
        p.compose.as_ref().unwrap().body.text,
        "",
        "the row types nothing"
    );
    press(&mut p, NamedKey::Backspace);
    assert!(p.compose.as_ref().unwrap().files.is_empty());
    assert_eq!(p.note, "took notes.pdf off");
}

#[test]
fn a_folder_a_missing_file_and_too_much_are_refused() {
    let (mut p, _) = reading();
    typed(&mut p, "r");
    let (d, big) = file("film.mov", 26 * 1024 * 1024);
    assert!(p
        .attach(d.path())
        .ends_with("is a folder \u{2014} attach the files in it"));
    assert!(p
        .attach(&d.path().join("gone.txt"))
        .starts_with("could not read"));
    assert_eq!(
        p.attach(&big),
        "film.mov would make 26 MB, more than the 25 MB Gmail sends"
    );
    assert!(p.compose.as_ref().unwrap().files.is_empty());
}

#[test]
fn the_chooser_answers_onto_the_row_and_a_click_on_attach_asks_it() {
    let (mut p, _) = reading();
    let (_d, path) = file("plan.txt", 10);
    typed(&mut p, "r");
    crate::inbox::pick::CHOSEN.with(|c| *c.borrow_mut() = vec![path.clone()]);
    let b = p.cards(80, 30).reply.expect("the box's card");
    assert!(p.compose_click(80, 30, b.row + b.h - 2, 5), "on + attach");
    assert!(p.picked());
    assert_eq!(p.compose.as_ref().unwrap().files[0].name, "plan.txt");
    assert_eq!(on(&p), On::Files);
    // The foot changed with the caret (and a note), so the box moved.
    let b = p.cards(80, 30).reply.unwrap();
    let (row, col) = (b.row + b.h - 2, b.col + 3 + "plan.txt 10 B ".len() as u16);
    assert!(p.compose_click(80, 30, row, col), "on its \u{2715}");
    assert!(p.compose.as_ref().unwrap().files.is_empty());
}

#[test]
fn a_reply_with_a_file_goes_in_its_thread_built_by_the_worker() {
    let (mut p, asks) = reading();
    let (_d, path) = file("menu.pdf", 5);
    typed(&mut p, "r");
    typed(&mut p, "Here it is.");
    p.attach(&path);
    p.send_reply();
    let mail = asks.try_iter().find_map(|a| match a {
        Ask::Mail(o) => Some(o),
        _ => None,
    });
    let o = mail.expect("built by the worker");
    assert_eq!(
        o.thread_id.as_deref(),
        Some("t1"),
        "a reply stays in its thread"
    );
    assert!(o.head.iter().all(|h| !h.starts_with("Bcc")), "{:?}", o.head);
    assert_eq!((o.words.as_str(), o.files.len()), ("Here it is.", 1));
}

#[test]
fn a_type_is_named_by_the_extension_and_the_chooser_prints_paths() {
    assert_eq!(
        mime_of("Q4.XLSX"),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    );
    assert_eq!(mime_of("noext"), "application/octet-stream");
    // Absolute as this system writes it: a Windows path has a drive.
    let (a, d) = match cfg!(windows) {
        true => (r"C:\a\b c.pdf", r"C:\d.png"),
        false => ("/a/b c.pdf", "/d.png"),
    };
    let got = crate::inbox::pick::paths(&format!("{a}\n\nrelative.txt\n{d}\n"));
    assert_eq!(got, [PathBuf::from(a), PathBuf::from(d)]);
    let names: Vec<String> = crate::inbox::pick::commands()
        .iter()
        .map(|c| c.get_program().to_string_lossy().into_owned())
        .collect();
    assert!(!names.is_empty(), "a chooser to try on every system");
}

/// Several files chosen at once said only the last file's line, so a file
/// refused mid-way went unseen: every refusal is said, after what went on.
#[test]
fn every_file_the_chooser_could_not_attach_is_said() {
    let (mut p, _) = reading();
    typed(&mut p, "r");
    let (_a, one) = file("a.pdf", 1024);
    let (_b, two) = file("b.pdf", 1024);
    let (_c, big) = file("film.mov", 26 * 1024 * 1024);
    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(Ok(vec![one, big, two])).unwrap();
    p.compose.as_mut().unwrap().picking = Some(rx);
    assert!(p.picked());
    assert_eq!(p.compose.as_ref().unwrap().files.len(), 2);
    assert!(p.note.starts_with("attached 2 files"), "{}", p.note);
    assert!(p.note.contains("film.mov would make"), "{}", p.note);
}
