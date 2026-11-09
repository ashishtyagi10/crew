//! A fresh list fills itself to [`WARM`] threads, and `j` asks for the next
//! page [`AHEAD`] rows before the end.
use std::sync::mpsc;

use winit::keyboard::{Key, NamedKey};

use super::{AHEAD, WARM};
use crate::inbox::wire::Thread;
use crate::inbox::worker::{Ask, Got, Worker};
use crate::inbox::{InboxPane, Stage};

fn page(n: std::ops::Range<usize>, more: bool) -> Got {
    Got::Page {
        query: String::new(),
        rows: n
            .map(|i| Thread {
                id: format!("t{i}"),
                at_ms: -(i as i64),
                ..Thread::default()
            })
            .collect(),
        next: Some("n".into()),
        more,
        whole: false,
    }
}

#[test]
fn a_fresh_list_fills_itself_to_warm_then_waits_for_j() {
    let (tx, asks) = mpsc::channel();
    let (answer, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    answer.send(page(0..50, false)).unwrap();
    p.poll();
    assert!(
        matches!(asks.try_recv(), Ok(Ask::More)),
        "one page is not enough"
    );
    answer.send(page(50..60, true)).unwrap();
    p.poll();
    assert_eq!(p.rows.len(), 60);
    assert!(
        60 < WARM,
        "a further page never asks by itself, short or not"
    );
    assert!(asks.try_recv().is_err());
    p.sel = 60 - AHEAD - 1;
    p.press(&Key::Named(NamedKey::ArrowDown), 80, 24);
    assert!(
        matches!(asks.try_recv(), Ok(Ask::More)),
        "near the end, ahead of it"
    );
}
