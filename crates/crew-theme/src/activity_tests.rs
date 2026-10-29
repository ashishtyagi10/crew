//! The activity dot against the accent. Both are drawn side by side — the
//! nav marks a working pane with a spinner in the accent and a pane with
//! unread output with a dot in `activity` — and nothing measured the pair
//! until 2026-10-28, when `nebula` (Δ 0.012) and `monokai` (Δ 0.009) turned
//! out to draw both in one colour, and `everforest`'s dot was its frame's
//! green (Δ 0.007). The same three visible steps as the alarm ([`FLOOR`]).
use super::*;
use crate::{ThemeId, ALL_THEMES};

#[test]
fn the_activity_dot_is_not_the_accent_on_any_coloured_page() {
    for id in ALL_THEMES.into_iter().filter(|id| !id.is_crt()) {
        let t = id.theme();
        let d = oklch::distance(t.activity, t.accent_default);
        assert!(
            d >= FLOOR - 1e-3,
            "{}: activity is only Δ {d:.4} from the accent (floor {FLOOR}) — \
             a pane with news reads as a pane at work",
            id.as_str()
        );
    }
}

/// The exemption, pinned like the alarm's: a phosphor tube has one hue, so
/// its dot and its accent can only differ by brightness.
#[test]
fn only_a_tube_draws_its_activity_in_its_accent() {
    let exempt: Vec<&str> = ALL_THEMES
        .into_iter()
        .filter(|id| oklch::distance(id.theme().activity, id.theme().accent_default) < FLOOR)
        .map(ThemeId::as_str)
        .collect();
    assert_eq!(exempt, vec!["crt-green"], "the exempt set has changed");
}

/// A dot drawn in the frame's colour on the frame's own line is no dot.
#[test]
fn the_activity_dot_stands_off_the_focused_frame() {
    for id in ALL_THEMES.into_iter().filter(|id| !id.is_crt()) {
        let t = id.theme();
        let d = oklch::distance(t.activity, t.border_focused);
        assert!(
            d >= FLOOR - 1e-3,
            "{}: activity is only Δ {d:.4} from the focused frame",
            id.as_str()
        );
    }
}
