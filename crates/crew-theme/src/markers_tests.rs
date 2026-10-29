//! Every pair of markers, not just the pairs someone once caught. The alarm
//! was held off the status (`signal_tests`) and the activity dot off the
//! accent and the frame (`activity_tests`), and the other pairs went
//! unmeasured until 2026-10-29: `everforest`'s alarm sat Δ 0.051 from its
//! broadcast, `amethyst`'s progress Δ 0.054 from its accent and
//! `glass-clear`'s alarm Δ 0.070 from its accent. All five are drawn in the
//! nav and on the cards side by side, so all ten pairs answer to [`FLOOR`].
use super::*;
use crate::{Theme, ThemeId, ALL_THEMES};

fn markers(t: &Theme) -> [(&'static str, (u8, u8, u8)); 5] {
    [
        ("accent", t.accent_default),
        ("status", t.status_fg),
        ("broadcast", t.broadcast),
        ("activity", t.activity),
        ("bell", t.bell),
    ]
}

/// The closest pair of markers on `t`, and how close.
fn closest(t: &Theme) -> (f32, &'static str, &'static str) {
    let m = markers(t);
    let mut out = (f32::MAX, "", "");
    for i in 0..m.len() {
        for j in i + 1..m.len() {
            let d = oklch::distance(m[i].1, m[j].1);
            if d < out.0 {
                out = (d, m[i].0, m[j].0);
            }
        }
    }
    out
}

#[test]
fn no_two_markers_share_a_colour_on_any_coloured_page() {
    for id in ALL_THEMES.into_iter().filter(|id| !id.is_crt()) {
        let (d, a, b) = closest(id.theme());
        assert!(
            d >= FLOOR - 1e-3,
            "{}: {a} and {b} are only Δ {d:.4} apart (floor {FLOOR})",
            id.as_str()
        );
    }
}

/// The tubes are exempt for the reason the alarm is: one hue, told apart by
/// brightness and the blink. Pinned to the tubes so a coloured palette
/// cannot drift in under their excuse.
#[test]
fn only_the_tubes_draw_two_markers_alike() {
    let exempt: Vec<ThemeId> = ALL_THEMES
        .into_iter()
        .filter(|id| closest(id.theme()).0 < FLOOR)
        .collect();
    assert!(exempt.iter().all(|id| id.is_crt()), "{exempt:?}");
}
