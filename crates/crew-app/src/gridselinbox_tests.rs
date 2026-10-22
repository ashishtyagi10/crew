//! A drag over an inbox pane, through the pane: what is lit is what is
//! copied, and both keep to the card the drag began in.
use std::sync::mpsc;

use super::{highlight, keeps, CellSel};
use crate::inbox::api::Mail;
use crate::inbox::rich::Rich;
use crate::inbox::wire::Thread;
use crate::inbox::worker::Worker;
use crate::inbox::{InboxPane, Reading, Stage};
use crate::pane::{Pane, PaneContent};

fn pane() -> Pane {
    let (tx, _) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.rows = vec![Thread {
        id: "t".into(),
        subject: "about t".into(),
        ..Thread::default()
    }];
    let mail = Mail {
        rich: Rich::of_plain("First line.\n\nSecond line.", None),
        ..Mail::default()
    };
    p.reading = Some(Reading {
        mails: Some(vec![mail]),
        ..Reading::new(p.rows[0].clone())
    });
    Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Inbox(Box::new(p)),
        grid: crew_term::GridSize {
            cols: 150,
            rows: 30,
        },
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
    }
}

#[test]
fn what_a_drag_lights_is_what_it_copies_inside_its_card() {
    let _g = crate::app::motion_test_guard();
    let pane = pane();
    let PaneContent::Inbox(p) = &pane.content else {
        unreachable!()
    };
    let mail = p.cards(150, 30).mail.unwrap();
    let sel = CellSel {
        pane: 0,
        anchor: (mail.col + 3, mail.row + 1),
        cursor: (149, mail.row + 8),
    };
    let lit = (1, 2, 3);
    let mut cells = pane.cells(false);
    highlight(&mut cells, &sel, lit, &pane);
    let keep = keeps(&pane, &sel);
    for c in cells.iter().filter(|c| c.bg == lit) {
        assert!(keep(c.col, c.row), "lit and kept: {} {}", c.row, c.col);
        assert!(c.c != '\u{2502}', "no edge is lit");
        assert!(c.col > mail.col && c.col < mail.col + mail.w - 1);
    }
    assert!(
        cells.iter().any(|c| c.bg == lit && c.c == 'S'),
        "the words are"
    );
    let list = p.cards(150, 30).list.unwrap();
    assert!(
        !keep(list.col + 3, mail.row + 2),
        "the list beside it is not"
    );
}
