use std::sync::mpsc;

use super::Density;
use crate::inbox::wire::Thread;
use crate::inbox::worker::Worker;
use crate::inbox::{InboxPane, Stage};

fn pane(n: usize) -> InboxPane {
    let (tx, _) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.rows = (0..n)
        .map(|i| Thread {
            id: format!("t{i}"),
            from: format!("P{i} <p{i}@x.org>"),
            subject: format!("subject {i}"),
            count: 1,
            ..Thread::default()
        })
        .collect();
    p
}

#[test]
fn two_lines_only_when_eight_of_them_fit_and_l_picks_for_the_session() {
    assert!(Density::Auto.two(17), "eight two-line rows over the foot");
    assert!(!Density::Auto.two(16), "seven: one line each");
    let mut p = pane(30);
    assert_eq!(p.shown(17), (0, 2, 8));
    assert_eq!(p.shown(16), (0, 1, 15));
    p.flip_density(16);
    assert_eq!(p.density, Density::Two, "L on a short pane asks for two");
    assert_eq!(p.shown(16), (0, 2, 7));
    p.flip_density(16);
    assert_eq!(p.density, Density::One);
    assert_eq!(
        Density::picked(),
        Density::One,
        "the pick outlives the pane"
    );
    super::PICKED.store(0, std::sync::atomic::Ordering::Relaxed);
}

#[test]
fn the_selection_stays_in_view_and_wears_the_bar_on_both_lines() {
    let _g = crate::app::theme_test_guard();
    let mut p = pane(30);
    p.sel = 20;
    let (top, lines, n) = p.shown(21);
    assert_eq!(
        (top, lines, n),
        (11, 2, 10),
        "the selected row is the last shown"
    );
    let bar = p.list_paint(60, 21).pop().expect("a bar");
    assert_eq!(
        (bar.y.floor(), bar.h.ceil()),
        (18.0, 2.0),
        "on rows 18 and 19"
    );
    let cells = p.cells(60, 21);
    assert!(
        cells
            .iter()
            .any(|c| c.row == 19 && c.c == ' ' && c.bg != crew_theme::theme().page_bg),
        "the wash covers the selected thread's second line"
    );
    let rules = p.list_paint(60, 21).len() - 1;
    assert_eq!(
        rules,
        9 - 1,
        "a hairline between threads, none against the wash"
    );
}
