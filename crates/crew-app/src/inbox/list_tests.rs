use std::sync::mpsc;

use super::Density;
use crate::inbox::layout::Slot;
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

/// The rows of `p` laid out in a pane `rows` tall, as `(row, what)` with a
/// thread written `#i` and a band by its name.
fn slots(p: &InboxPane, rows: u16) -> Vec<(u16, String)> {
    p.shown(rows)
        .slots
        .into_iter()
        .map(|(r, s)| match s {
            Slot::Band(b) => (r, b),
            Slot::Thread(i) => (r, format!("#{i}")),
            Slot::Fold(n) => (r, format!("fold {n}")),
        })
        .collect()
}

#[test]
fn two_lines_only_when_six_fit_with_air_and_l_picks_for_the_session() {
    let _g = crate::app::motion_test_guard();
    assert!(
        Density::Auto.two(20),
        "six two-line rows, a blank under each"
    );
    assert!(!Density::Auto.two(19), "five: one line each");
    let mut p = pane(30);
    let shown = p.shown(20);
    let rows: Vec<u16> = shown.threads().map(|(r, _)| r).collect();
    assert_eq!(shown.lines, 2);
    assert_eq!(
        rows,
        [2, 5, 8, 11, 14, 17],
        "under a row of air and the band, a blank row between threads"
    );
    assert_eq!(
        p.shown(19).threads().count(),
        16,
        "one line, no air between"
    );
    p.flip_density(19);
    assert_eq!(p.density, Density::Two, "L on a short pane asks for two");
    assert_eq!(p.shown(19).lines, 2);
    p.flip_density(19);
    assert_eq!(p.density, Density::One);
    assert_eq!(
        Density::picked(),
        Density::One,
        "the pick outlives the pane"
    );
    super::PICKED.store(0, std::sync::atomic::Ordering::Relaxed);
}

#[test]
fn each_day_gets_a_band_after_a_blank_row_and_the_selection_stays_in_view() {
    let _g = crate::app::motion_test_guard();
    let now = chrono::Utc::now().timestamp_millis();
    let mut p = pane(12);
    for (i, t) in p.rows.iter_mut().enumerate() {
        // Three today, three yesterday, then a year back.
        let days = [0, 0, 0, 1, 1, 1, 400][i.min(6)];
        t.at_ms = now - days * 86_400_000 - i as i64 * 60_000;
    }
    p.density = Density::One;
    let got = slots(&p, 14);
    let year = crate::inbox::days::band(p.rows[6].at_ms, now);
    assert_eq!(got[0], (1, "today".to_string()), "under a row of air");
    assert_eq!(
        got[4],
        (6, "yesterday".to_string()),
        "a blank row before a day"
    );
    assert_eq!(got[8], (11, year));
    assert_eq!(
        got.last(),
        Some(&(12, "#6".to_string())),
        "a row of air over the foot"
    );
    p.sel = 9;
    let got = slots(&p, 14);
    assert!(
        got.iter().any(|(_, s)| s == "#9"),
        "the selected thread is on screen: {got:?}"
    );
    assert!(
        matches!(&got[0], (1, b) if !b.starts_with('#')),
        "the first row is a band: {got:?}"
    );
    p.density = Density::Two;
    for sel in 0..12 {
        p.sel = sel;
        let shown = p.shown(14);
        let at = shown.threads().find(|&(_, i)| i == sel);
        assert!(
            at.is_some_and(|(r, _)| r + 2 <= 13),
            "{sel}: {:?}",
            shown.slots
        );
    }
}

#[test]
fn the_list_paints_nothing_over_its_rows() {
    let mut p = pane(30);
    (p.sel, p.density) = (3, Density::Two);
    assert!(
        p.list_paint(60, 21, 2.0).is_empty(),
        "air divides the threads"
    );
    let cells = p.cells(60, 21);
    let washed = |r: u16| cells.iter().any(|c| c.row == r && c.c == ' ');
    let inner = p.cards(60, 21).list.expect("the list's card").inner();
    let shown = p.shown(inner.h);
    let (r, _) = shown.threads().find(|&(_, i)| i == 3).unwrap();
    let r = inner.row + r;
    assert!(
        washed(r) && washed(r + 1),
        "the wash covers both of its lines"
    );
}
