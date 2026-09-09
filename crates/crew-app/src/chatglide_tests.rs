use super::*;
use crate::chatlayout::Message;
use crate::motion::{set_level, MotionLevel};

fn msg(sender: &str, text: &str, ts: &str) -> Message {
    Message {
        sender: sender.into(),
        text: text.into(),
        ts: ts.into(),
        meta: String::new(),
        usage: None,
        expanded: false,
    }
}

/// The row of the first placed line whose text contains `needle`.
fn row_of(placed: &[(u16, CardLine)], needle: &str) -> u16 {
    placed
        .iter()
        .find(|(_, l)| l.iter().map(|c| c.c).collect::<String>().contains(needle))
        .unwrap_or_else(|| panic!("{needle:?} not placed"))
        .0
}

#[test]
fn offset_is_one_row_through_the_glide_then_zero() {
    let full = MotionLevel::Full;
    assert_eq!(offset_rows("5000", 5_000, full), 1, "just landed");
    assert_eq!(
        offset_rows("5000", 5_000 + GLIDE_MS / 2, full),
        1,
        "mid-glide"
    );
    assert_eq!(offset_rows("5000", 5_000 + GLIDE_MS, full), 0, "in place");
    // The eased progress the fade beside it shares: 0 → 1, fast departure.
    let (a, b, c) = (
        glide_t("5000", 5_000, full),
        glide_t("5000", 5_000 + GLIDE_MS / 2, full),
        glide_t("5000", 5_000 + GLIDE_MS, full),
    );
    assert!(a == 0.0 && a < b && b < c && c == 1.0, "{a} {b} {c}");
    assert!(b > 0.8, "ease-out: most of the travel is early: {b}");
    // Subtle: the same slide over 60% of the time.
    let subtle = MotionLevel::Subtle;
    assert_eq!(offset_rows("5000", 5_100, subtle), 1);
    assert_eq!(offset_rows("5000", 5_132, subtle), 0);
    // Off is a true static path; so are the counting pass and a bad stamp.
    assert_eq!(offset_rows("5000", 5_000, MotionLevel::Off), 0);
    assert_eq!(glide_t("5000", 5_000, MotionLevel::Off), 1.0);
    assert_eq!(offset_rows("5000", 0, full), 0);
    assert_eq!(offset_rows("", 5_000, full), 0);
}

#[test]
fn the_newest_settled_card_lands_one_row_low_and_nothing_above_moves() {
    let _g = crate::app::motion_test_guard();
    set_level(MotionLevel::Full);
    let (a, b) = (msg("user", "hello", "1000"), msg("coder", "world", "5000"));
    let msgs = [&a, &b];
    let fresh = place(&msgs, 40, 12, 1, 0, View::default(), 5_000);
    let settled = place(&msgs, 40, 12, 1, 0, View::default(), 15_000);
    assert_eq!(
        row_of(&fresh, "hello"),
        row_of(&settled, "hello"),
        "above: still"
    );
    assert_eq!(
        row_of(&fresh, "coder"),
        row_of(&settled, "coder") + 1,
        "the new card sits one row low"
    );
    assert!(
        fresh.iter().all(|(r, _)| *r <= 12),
        "clipped at the window's last row"
    );
    assert_eq!(
        fresh.len(),
        settled.len() - 1,
        "its last line is off the bottom"
    );
}

#[test]
fn no_glide_when_scrolled_up_streaming_or_off() {
    let _g = crate::app::motion_test_guard();
    set_level(MotionLevel::Full);
    let (a, b) = (msg("user", "hello", "1000"), msg("coder", "world", "5000"));
    let msgs = [&a, &b];
    let rows = |placed: &[(u16, CardLine)]| placed.iter().map(|(r, _)| *r).collect::<Vec<_>>();
    // Scrolled up: rows above would jump, so nothing moves.
    assert_eq!(
        rows(&place(&msgs, 40, 3, 1, 1, View::default(), 5_000)),
        rows(&place(&msgs, 40, 3, 1, 1, View::default(), 15_000))
    );
    // Still streaming: it has been on screen typing — not an arrival.
    let streaming = View {
        streaming_from: 1,
        ..View::default()
    };
    assert_eq!(
        rows(&place(&msgs, 40, 12, 1, 0, streaming, 5_000)),
        rows(&place(&msgs, 40, 12, 1, 0, streaming, 15_000))
    );
    set_level(MotionLevel::Off);
    assert_eq!(
        rows(&place(&msgs, 40, 12, 1, 0, View::default(), 5_000)),
        rows(&place(&msgs, 40, 12, 1, 0, View::default(), 15_000))
    );
}
