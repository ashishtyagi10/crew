use super::LINK_LC;
use crate::{apca, readable::link, ALL_THEMES};

#[test]
fn a_dark_page_s_link_reads_as_body_text() {
    for id in ALL_THEMES {
        let t = id.theme();
        if !t.dark || t.is_tube() {
            continue;
        }
        let lc = apca::lc(link(t), t.term_bg);
        // Before (2026-11-18): 52.7 on mercury, 53-54 on four more.
        assert!(lc >= LINK_LC, "{} link Lc {lc:.1}", id.as_str());
        // Still a link, not the prose: a step under the ink.
        assert!(lc + 15.0 < apca::lc(t.ink, t.term_bg), "{}", id.as_str());
    }
}

#[test]
fn the_lift_keeps_the_blue() {
    for id in ALL_THEMES {
        let t = id.theme();
        let (k, b) = (
            crate::oklch::from_srgb(link(t)),
            crate::oklch::from_srgb((90, 170, 255)),
        );
        if t.is_tube() {
            continue;
        }
        let d = (k.h - b.h + 540.0).rem_euclid(360.0) - 180.0;
        assert!(
            d.abs() < 10.0,
            "{} link hue {:.0} vs {:.0}",
            id.as_str(),
            k.h,
            b.h
        );
    }
}
