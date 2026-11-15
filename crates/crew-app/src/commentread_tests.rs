use super::*;

/// Every dark page that is not a tube draws its comments at the floor or
/// as close to it as the rung under code allows.
#[test]
fn a_dark_pages_comment_reads_on_its_field() {
    for id in crew_theme::ALL_THEMES {
        let t = id.theme();
        if !t.dark || t.is_tube() {
            continue;
        }
        let d = crate::chatink::derive(t);
        let lc = apca::lc(d.comment, d.code_bg);
        let rung = contrast_ratio(d.code, d.comment);
        assert!(
            lc >= COMMENT_LC - 0.5 || rung < RUNG + 0.05,
            "{}: comment Lc {lc:.1}, code {rung:.2}× above it",
            id.as_str()
        );
        assert!(
            rung >= 1.15,
            "{}: code only {rung:.2}× over its comment",
            id.as_str()
        );
    }
}

/// A light page and a tube keep the comment they had.
#[test]
fn a_light_page_and_a_tube_are_left_alone() {
    for id in crew_theme::ALL_THEMES {
        let t = id.theme();
        if t.dark && !t.is_tube() {
            continue;
        }
        let c = (90, 90, 90);
        assert_eq!(lift(c, t, t.ink, t.page_bg), c, "{}", id.as_str());
    }
}
