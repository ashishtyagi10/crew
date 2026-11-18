use super::READ_LC;
use crate::readable::{danger, link, warn};
use crate::{apca, oklch, ALL_THEMES};

/// Hue angle distance in degrees.
fn gap(a: (u8, u8, u8), b: (u8, u8, u8)) -> f32 {
    let (a, b) = (oklch::from_srgb(a).h, oklch::from_srgb(b).h);
    ((a - b + 540.0).rem_euclid(360.0) - 180.0).abs()
}

#[test]
fn a_dark_page_s_link_and_danger_read_as_body_text() {
    for id in ALL_THEMES {
        let t = id.theme();
        if !t.dark || t.is_tube() {
            continue;
        }
        // Before (2026-11-18): link 52.7 on mercury, danger 38.1 there.
        let (k, d) = (apca::lc(link(t), t.term_bg), apca::lc(danger(t), t.page_bg));
        assert!(k >= READ_LC, "{} link Lc {k:.1}", id.as_str());
        assert!(d >= READ_LC, "{} danger Lc {d:.1}", id.as_str());
        // Still a colour, not the prose: a step under the ink.
        assert!(k + 15.0 < apca::lc(t.ink, t.term_bg), "{}", id.as_str());
        // The alarm is no quieter than the caution, give or take.
        assert!(d + 8.0 > apca::lc(warn(t), t.page_bg), "{}", id.as_str());
    }
}

#[test]
fn the_lift_keeps_each_hue() {
    for id in ALL_THEMES {
        let t = id.theme();
        if t.is_tube() {
            continue;
        }
        let k = gap(link(t), (90, 170, 255));
        assert!(k < 10.0, "{} link hue {k:.0}° off", id.as_str());
        let d = gap(danger(t), (230, 90, 90));
        assert!(d < 12.0, "{} danger hue {d:.0}° off", id.as_str());
    }
}
