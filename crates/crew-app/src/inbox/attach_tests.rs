use std::sync::mpsc;

use super::super::api::Mail;
use super::super::body::Part;
use super::super::wire::Thread;
use super::super::worker::{Ask, Got, Worker};
use super::super::{InboxPane, Reading, Stage};

fn pane() -> (InboxPane, mpsc::Receiver<Ask>) {
    let (tx, asks) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    let attached = vec![
        Part {
            name: "report.pdf".into(),
            att: Some("A1".into()),
            size: 12_400,
            ..Part::default()
        },
        Part {
            name: "../notes.txt".into(),
            data: Some("aGk".into()),
            size: 2,
            ..Part::default()
        },
    ];
    p.reading = Some(Reading {
        mails: Some(vec![Mail {
            id: "m1".into(),
            text: "See attached.".into(),
            attached,
            ..Mail::default()
        }]),
        ..Reading::new(Thread::default())
    });
    (p, asks)
}

fn shown(p: &InboxPane) -> String {
    let mut rows = vec![String::new(); 30];
    let mut cells = p.cells(80, 30);
    cells.sort_by_key(|c| (c.row, c.col));
    for c in cells {
        rows[c.row as usize].push(c.c);
    }
    rows.join("\n")
}

#[test]
fn an_attachment_is_a_row_whose_link_saves_that_file() {
    let (mut p, asks) = pane();
    assert!(
        shown(&p).contains("\u{2193} report.pdf \u{b7} 12 KB"),
        "{}",
        shown(&p)
    );
    let links: Vec<String> = p.links(80, 30).into_iter().map(|(_, _, u)| u).collect();
    assert_eq!(links, ["crew-attachment:m1/0", "crew-attachment:m1/1"]);
    assert_eq!(p.save_attachment(&links[1]), "saving notes.txt\u{2026}");
    match asks.try_recv() {
        Ok(Ask::Save {
            msg,
            att,
            data,
            name,
        }) => {
            assert_eq!(
                (msg.as_str(), att, data.as_deref(), name.as_str()),
                ("m1", None, Some("aGk"), "../notes.txt")
            );
        }
        _ => panic!("no save asked"),
    }
    assert!(!p.busy, "a save is not a load the list waits on");
    assert_eq!(
        p.save_attachment("crew-attachment:m9/0"),
        "that attachment is not in the open thread"
    );
}

#[test]
fn a_saved_file_is_opened_and_a_failed_save_says_why() {
    let (mut p, _asks) = pane();
    let path = std::env::temp_dir().join("report.pdf");
    p.take(Got::Saved(Ok(path.clone())));
    assert!(
        p.note.starts_with("saved report.pdf to Downloads"),
        "{}",
        p.note
    );
    let opened = crate::linkclick::OPENED.with(|o| o.borrow().clone());
    assert!(
        opened.contains(&path.to_string_lossy().into_owned()),
        "{opened:?}"
    );
    p.take(Got::Saved(Err("no Downloads folder".into())));
    assert_eq!(p.note, "no Downloads folder");
}

#[test]
fn w_asks_for_a_window_and_the_windows_look_moves_with_the_pane() {
    let (mut p, _asks) = pane();
    let grid = crew_term::GridSize { cols: 80, rows: 30 };
    let before = crate::inbox::mailwin::look(&p, grid);
    p.press(&winit::keyboard::Key::Character("w".into()), 80, 30);
    assert!(p.reading.as_ref().unwrap().pop_out);
    p.pics.set("m1|cid:x".into(), None);
    assert_ne!(
        crate::inbox::mailwin::look(&p, grid),
        before,
        "a picture landing redraws it"
    );
    let still = crate::inbox::mailwin::look(&p, grid);
    p.press(
        &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
        80,
        30,
    );
    assert_ne!(
        crate::inbox::mailwin::look(&p, grid),
        still,
        "leaving the thread redraws it"
    );
}
