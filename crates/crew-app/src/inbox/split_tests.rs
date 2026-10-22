use std::sync::mpsc;

use crate::inbox::api::Mail;
use crate::inbox::rich::Rich;
use crate::inbox::wire::Thread;
use crate::inbox::worker::Worker;
use crate::inbox::{InboxPane, Reading, Stage};

/// A pane listing three threads, reading the second: a link in its words.
fn reading() -> InboxPane {
    let (tx, _) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.rows = ["a", "b", "c"]
        .iter()
        .map(|id| Thread {
            id: id.to_string(),
            from: format!("{id} <{id}@x.org>"),
            subject: format!("about {id}"),
            ..Thread::default()
        })
        .collect();
    p.sel = 1;
    let rich = Rich::of_html("<p>See <a href='https://x.io/t'>Track it</a> now</p>", None);
    p.reading = Some(Reading {
        mails: Some(vec![Mail {
            rich,
            ..Mail::default()
        }]),
        ..Reading::new(p.rows[1].clone())
    });
    p
}

#[test]
fn a_wide_pane_reads_beside_the_list_and_a_narrow_one_as_before() {
    let _g = crate::app::motion_test_guard();
    let mut p = reading();
    assert_eq!(p.split(139), None);
    let s = p.split(150).expect("wide enough");
    assert_eq!((s.list, s.at), (60, 61), "two fifths, then the air");
    assert_eq!(p.split(400).unwrap().list, 64, "never wider than 64");
    assert_eq!(p.reader_box(120), (2, 116), "inside the card");
    assert_eq!(p.reader_box(150), (62, 86));
    let body = p.body_rows(150, 30);
    let cells: Vec<_> = p
        .cells(150, 30)
        .into_iter()
        .filter(|c| c.row < body)
        .collect();
    let text = |lo: u16, hi: u16| -> String {
        let mut on: Vec<_> = cells
            .iter()
            .filter(|c| (lo..hi).contains(&c.col) && c.c != ' ')
            .collect();
        on.sort_by_key(|c| (c.row, c.col));
        on.iter().map(|c| c.c).collect()
    };
    assert!(text(0, 60).contains("aboutb"), "the list on the left");
    assert!(text(61, 150).contains("Trackit"), "the thread on the right");
    assert!(!text(0, 61).contains("Track"), "and not on the left");
    p.reading = None;
    assert_eq!(
        p.split(150),
        None,
        "no thread open, the list takes the pane"
    );
}

#[test]
fn links_by_click_and_by_hint_are_where_the_right_half_draws_them() {
    let _g = crate::app::motion_test_guard();
    let p = reading();
    let links = p.links(150, 30);
    let (row, col, url) = links.first().cloned().expect("the link");
    assert!(col > 63, "{col}");
    assert_eq!(
        p.link_at(150, 30, row, col + 2).as_deref(),
        Some(url.as_str())
    );
    assert_eq!(p.link_at(150, 30, row, 10), None, "the list's half");
    let narrow = p.links(100, 30);
    let moved = p.reader_box(150).0 - p.reader_box(100).0;
    assert_eq!(
        narrow[0].1 + moved,
        col,
        "the same words, moved with the half"
    );
}

#[test]
fn a_drag_copies_the_inside_of_its_card_and_nothing_beside_it() {
    let _g = crate::app::motion_test_guard();
    let p = reading();
    let mail = p.cards(150, 30).mail.unwrap().inner();
    // From the mail's first row to well below its words, across the pane.
    let sel = crate::gridsel::CellSel {
        pane: 0,
        anchor: (mail.col + 1, mail.row),
        cursor: (149, mail.row + 6),
    };
    let text = crate::gridsel::selection_text(&p.copy_cells(150, 30, sel.anchor), &sel);
    assert!(text.contains("Track it"), "{text}");
    assert!(!text.contains('\u{2502}'), "no card's edge: {text}");
    assert!(
        !text.contains("about a"),
        "none of the list beside it: {text}"
    );
    let all = crate::gridsel::selection_text(&p.cells(150, 30), &sel);
    assert!(
        all.contains('\u{2502}'),
        "the pane as drawn does have edges"
    );
}
