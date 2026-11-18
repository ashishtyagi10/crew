//! A dark page's quiet rungs, held in APCA Lc where `rampdark` floors them.
use crate::ramp::Ramp;

#[test]
fn a_dark_page_s_quiet_text_reads_as_secondary_text() {
    let mut bad = Vec::new();
    for id in crate::ALL_THEMES {
        let t = id.theme();
        if !t.dark || t.is_tube() {
            continue;
        }
        let r = Ramp::fitted(t);
        let lc = |c| crate::apca::lc(c, t.page_bg);
        // Before (2026-11-18): legend 40-45, hints 32-37, placeholder 28-32.
        for (name, c, floor) in [
            ("legend_off", r.legend_off(), 48.0),
            ("dim", r.dim(), 40.5),
            ("hint_fg", r.hint_fg(), 41.0),
            ("placeholder", r.placeholder(), 36.5),
        ] {
            if lc(c) < floor {
                bad.push(format!("{} {name} Lc {:.1} < {floor}", id.as_str(), lc(c)));
            }
        }
        // Still a step under the body text.
        assert!(
            lc(r.legend_off()) + 10.0 < lc(r.text_muted()),
            "{}",
            id.as_str()
        );
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn light_pages_and_tubes_keep_their_ladder() {
    for id in crate::ALL_THEMES {
        let t = id.theme();
        let name = Ramp::fitted(t).house().name;
        assert_eq!(
            name.contains("dark)"),
            t.dark && !t.is_tube(),
            "{}",
            id.as_str()
        );
    }
}
