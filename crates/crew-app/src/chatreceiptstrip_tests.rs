use super::*;
use crate::chatbody::CardLine;
use crate::chatreceipthead::rows;

const ONE: &str = "task #3: 1 file changed (+4 \u{2212}1): ~ src/lib.rs\n```diff\n-a\n+b\n```";

fn text(r: &CardLine) -> String {
    r.iter().map(|c| c.c).collect()
}

#[test]
fn widths_are_proportional_and_sum_exactly() {
    assert_eq!(widths([5_000, 5_000, 0, 10_000], 20), [5, 5, 0, 10]);
    for w in MIN..=MAX {
        for ms in [
            [1, 1, 1, 1],
            [7, 3, 11, 13],
            [1, 0, 0, 999],
            [333, 333, 334, 0],
        ] {
            assert_eq!(widths(ms, w).iter().sum::<usize>(), w, "{ms:?} in {w}");
        }
    }
}

#[test]
fn a_tiny_segment_is_dropped_not_rounded_up() {
    // 0.1 s of edit in a 60 s turn is under half a cell of 16.
    let got = widths([40_000, 10_000, 100, 9_900], 16);
    assert_eq!(got[2], 0);
    assert_eq!(got.iter().sum::<usize>(), 16);
}

#[test]
fn an_all_think_turn_is_one_run_and_no_time_is_none() {
    assert_eq!(widths([9_000, 0, 0, 0], 16), [16, 0, 0, 0]);
    assert_eq!(widths([0; 4], 16), [0; 4]);
}

#[test]
fn the_strip_lands_on_the_first_row_and_costs_no_row() {
    let _g = crate::app::theme_test_guard();
    let fg = (200, 200, 200);
    let mut r = rows(ONE, 80, fg).unwrap();
    let before = r.len();
    strip_on(&mut r, [20_000, 10_000, 2_000, 10_000], 80);
    assert_eq!(r.len(), before);
    let t = text(&r[0]);
    assert!(t.ends_with(" 42s · 47% thinking"), "{t}");
    assert_eq!(t.matches(BLOCK).count(), MAX);
    assert!(r[0].len() < 80);
}

#[test]
fn a_narrow_head_keeps_its_plain_row() {
    let _g = crate::app::theme_test_guard();
    let fg = (200, 200, 200);
    let mut r = rows(ONE, 40, fg).unwrap();
    let was = text(&r[0]);
    strip_on(&mut r, [20_000, 10_000, 2_000, 10_000], 40);
    assert_eq!(text(&r[0]), was);
    // and a turn under a second says nothing
    let mut r = rows(ONE, 80, fg).unwrap();
    let was = text(&r[0]);
    strip_on(&mut r, [200, 100, 0, 0], 80);
    assert_eq!(text(&r[0]), was);
}

#[test]
fn a_stamped_receipt_draws_its_strip_through_the_card_and_row_count_holds() {
    use crate::chatlayout::Message;
    use crate::chatmsgs::{full_body, View};
    let _g = crate::app::theme_test_guard();
    let m = |meta: String| Message {
        sender: "agent smith".into(),
        text: ONE.into(),
        ts: String::new(),
        meta,
        usage: None,
        expanded: true,
    };
    let plain = full_body(&m(String::new()), 80, View::default());
    let meta = crate::chatstrip::stamp("task:3", [20_000, 10_000, 2_000, 10_000]);
    let drawn = full_body(&m(meta), 80, View::default());
    assert_eq!(drawn.len(), plain.len());
    assert!(text(&drawn[0]).contains(" 42s · "), "{}", text(&drawn[0]));
}

#[test]
fn the_phase_that_held_most_is_named_with_its_share() {
    let (k, said) = most([1_000, 2_000, 0, 7_000]).unwrap();
    assert_eq!((k, said.as_str()), (Kind::Run, "70% running"));
}
