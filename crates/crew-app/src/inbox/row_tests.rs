use crew_render::CellView;

use super::{cells, who_w, Ctx, Fit};
use crate::inbox::wire::{Sender, Thread};

const NOW: i64 = 1_792_500_000_000;
const H: i64 = 3_600_000;

fn thread() -> Thread {
    let s = |f: &str, sent| Sender {
        from: f.into(),
        sent,
    };
    Thread {
        id: "a".into(),
        from: "Bo Park <bo@y.org>".into(),
        subject: "Q4 roadmap review".into(),
        snippet: "Can we move the review to Thursday?".into(),
        unread: true,
        starred: true,
        at_ms: NOW - 3 * H,
        count: 3,
        senders: vec![
            s("Ann Lee <ann@x.org>", false),
            s("Me <me@x.org>", true),
            s("Bo Park <bo@y.org>", false),
        ],
    }
}

fn draw(th: &Thread, cols: u16, two: bool, selected: bool) -> Vec<CellView> {
    let x = Ctx {
        cols,
        two,
        me: "",
        now: NOW,
        who_w: who_w(std::slice::from_ref(th), cols, ""),
    };
    let mut out = Vec::new();
    cells(&mut out, th, 0, selected, &x);
    out
}

fn text(cells: &[CellView], row: u16, cols: u16) -> String {
    let mut line = vec![' '; usize::from(cols)];
    for c in cells.iter().filter(|c| c.row == row) {
        line[usize::from(c.col)] = c.c;
    }
    line.into_iter().collect::<String>().trim_end().to_string()
}

#[test]
fn the_ladder_drops_the_snippet_then_the_chip_then_senders_then_shortens_the_time() {
    let th = thread();
    let one = |cols| text(&draw(&th, cols, false, false), 0, cols);
    let wide = one(120);
    assert!(
        wide.starts_with(" \u{25cf} Ann Lee, me, Bo Park 3"),
        "{wide:?}"
    );
    assert!(
        wide.contains("Q4 roadmap review \u{2014} Can we move"),
        "{wide:?}"
    );
    assert!(wide.ends_with("\u{2605} 3h"), "{wide:?}");
    let at80 = one(80);
    assert!(
        at80.contains(" \u{2014} Can we"),
        "the snippet stays: {at80:?}"
    );
    let at60 = one(60);
    assert!(
        !at60.contains('\u{2014}'),
        "the snippet goes first: {at60:?}"
    );
    assert!(
        at60.starts_with(" \u{25cf} Ann, me, Bo 3"),
        "first names, chip: {at60:?}"
    );
    let at40 = one(40);
    assert!(
        at40.starts_with(" \u{25cf} Bo Park  Q4"),
        "the newest, no chip: {at40:?}"
    );
    assert!(at40.ends_with("\u{2605} 3h"), "{at40:?}");
    assert_eq!(Fit::at(30, false).compact, true, "the time shortens last");
    for cols in [40, 60, 80, 120] {
        let c = draw(&th, cols, false, false);
        assert!(
            c.iter().all(|c| c.col < cols - 1),
            "the margin column at {cols}"
        );
    }
}

#[test]
fn two_lines_put_the_people_over_the_subject_and_climb_the_ladder_later() {
    let th = thread();
    for cols in [40, 60, 80, 120] {
        let c = draw(&th, cols, true, false);
        let (top, body) = (text(&c, 0, cols), text(&c, 1, cols));
        assert!(
            top.starts_with(" \u{25cf} Ann Lee, me, Bo Park 3"),
            "{cols}: {top:?}"
        );
        assert!(top.ends_with("\u{2605} 3h"), "{cols}: {top:?}");
        assert!(body.starts_with("   Q4 roadmap review"), "{cols}: {body:?}");
        assert_eq!(body.contains('\u{2014}'), cols >= 60, "{cols}: {body:?}");
    }
    let narrow = text(&draw(&th, 30, true, false), 0, 30);
    assert!(
        !narrow.contains("Park 3"),
        "the chip goes at 30: {narrow:?}"
    );
}

#[test]
fn the_selected_row_is_washed_edge_to_edge_and_its_ink_reads_on_it() {
    let _g = crate::app::theme_test_guard();
    let th = Thread {
        unread: false,
        ..thread()
    };
    let floor = crew_theme::contrast::text_floor();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let t = crew_theme::theme();
        let wash = crew_theme::readable::selection_bg(&t);
        let c = draw(&th, 60, true, true);
        for row in [0, 1] {
            let cols: Vec<u16> = c.iter().filter(|c| c.row == row).map(|c| c.col).collect();
            assert_eq!(
                cols,
                (0..59).collect::<Vec<_>>(),
                "{id:?}: line {row} is whole"
            );
        }
        let chip = c.iter().find(|c| c.mark).expect("the count is a chip");
        assert_ne!(chip.bg, wash, "a chip keeps its capsule on the wash");
        for cell in c.iter().filter(|c| c.c != ' ' && !c.mark) {
            assert_eq!(cell.bg, wash);
            let r = crew_theme::contrast_ratio(cell.fg, wash);
            assert!(
                r >= floor - 0.05,
                "{id:?}: {:?} reads {r:.2} on the wash",
                cell.c
            );
        }
        let plain = draw(&th, 60, true, false);
        assert!(
            plain.iter().all(|c| c.mark || c.bg == t.page_bg) && plain.len() < c.len(),
            "an unselected row draws only its words, on the page"
        );
        let subject = plain.iter().find(|c| c.c == 'Q').unwrap();
        assert_eq!(
            (subject.fg, subject.bold),
            (t.text_muted, false),
            "read mail recedes"
        );
    }
}

#[test]
fn every_marker_has_a_cell_of_its_own() {
    let c = draw(&thread(), 80, false, false);
    let at = |ch: char| c.iter().find(|c| c.c == ch).map(|c| c.col);
    let (dot, star) = (at('\u{25cf}').unwrap(), at('\u{2605}').unwrap());
    assert_eq!(dot, 1, "the dot has the gutter");
    assert!(star > 60, "the star sits by the time: {star}");
    let mut cols: Vec<u16> = c.iter().map(|c| c.col).collect();
    cols.sort_unstable();
    cols.dedup();
    assert_eq!(cols.len(), c.len(), "no two cells share a column");
}
