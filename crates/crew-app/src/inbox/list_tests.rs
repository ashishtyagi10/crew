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
        })
        .collect()
}

#[test]
fn two_lines_only_when_eight_of_them_fit_and_l_picks_for_the_session() {
    assert!(Density::Auto.two(17), "eight two-line rows over the foot");
    assert!(!Density::Auto.two(16), "seven: one line each");
    let mut p = pane(30);
    let shown = p.shown(17);
    assert_eq!(
        (shown.lines, shown.threads().count()),
        (2, 7),
        "a band, then seven"
    );
    assert_eq!(p.shown(16).threads().count(), 14);
    p.flip_density(16);
    assert_eq!(p.density, Density::Two, "L on a short pane asks for two");
    assert_eq!(p.shown(16).lines, 2);
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
fn each_day_gets_a_band_and_the_selection_stays_in_view_under_one() {
    let now = chrono::Utc::now().timestamp_millis();
    let mut p = pane(12);
    for (i, t) in p.rows.iter_mut().enumerate() {
        // Three today, three yesterday, then a year back.
        let days = [0, 0, 0, 1, 1, 1, 400][i.min(6)];
        t.at_ms = now - days * 86_400_000 - i as i64 * 60_000;
    }
    p.density = Density::One;
    let got = slots(&p, 12);
    let year = crate::inbox::days::band(p.rows[6].at_ms, now);
    assert_eq!(got[0], (0, "today".to_string()));
    assert_eq!(got[4], (4, "yesterday".to_string()));
    assert_eq!(got[8], (8, year), "a band where the day changes");
    assert_eq!(got.len(), 11, "a row is left for the foot: {got:?}");
    p.sel = 9;
    let got = slots(&p, 12);
    assert!(
        got.contains(&(10, "#9".to_string())),
        "the selected thread is the last: {got:?}"
    );
    assert!(
        matches!(&got[0], (0, b) if !b.starts_with('#')),
        "the first row is a band: {got:?}"
    );
    let rows: Vec<u16> = got.iter().map(|(r, _)| *r).collect();
    assert!(
        rows.windows(2).all(|w| w[1] == w[0] + 1),
        "no row skipped: {got:?}"
    );
}

#[test]
fn the_selection_wears_the_bar_on_both_lines_and_rules_stay_off_the_wash() {
    let _g = crate::app::theme_test_guard();
    let mut p = pane(30);
    let now = chrono::Utc::now().timestamp_millis();
    for t in &mut p.rows {
        t.at_ms = now;
    }
    p.sel = 3;
    let paint = p.list_paint(60, 21, 2.0);
    let bars: Vec<_> = paint
        .iter()
        .filter(|q| q.color == crate::palette::accent())
        .collect();
    assert_eq!(bars.len(), 1);
    assert_eq!(
        (bars[0].y.floor(), bars[0].h.ceil()),
        (7.0, 2.0),
        "under today's band, rows 7 and 8"
    );
    let cells = p.cells(60, 21);
    assert!(
        cells
            .iter()
            .any(|c| c.row == 8 && c.c == ' ' && c.bg != crew_theme::theme().page_bg),
        "the wash covers the selected thread's second line"
    );
    let rules = paint.len() - 1;
    assert_eq!(
        rules,
        1 + 9 - 1 - 2,
        "the band's rule; between threads, none at the wash"
    );
}
