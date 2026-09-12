//! The nav's RESTART card, shot off-screen. Its own file rather than a fourth
//! entry in `transientshot_tests`: that one was at the 200-line cap, and a
//! button that blinks is a different question from a card that reports
//! progress — this asks whether it still reads as pressable, in both blink
//! phases and at the narrowest nav a user can drag to.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew restart_shot -- --ignored --nocapture`
use crate::transientshot_tests::{card_shot, NAV_W};

/// The RESTART card, in both blink phases and at the narrowest nav a user can
/// drag to. It is a button, so the question a shot answers is whether it still
/// reads as one when the column is 150px wide and the word has to share its
/// row with the glyph.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn restart_shot_card() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let blink = crate::attention::BLINK_MS;
    for (name, w, now) in [
        ("restart-on", NAV_W, 0),
        ("restart-off", NAV_W, blink),
        ("restart-narrow", 150, 0),
    ] {
        let Some(rows) = card_shot(name, "RESTART", w, 90, |c, r| {
            crate::restartcard::restart_cells("0.22.22", now, c, r)
        }) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(
            rows[0].contains("restart"),
            "the button survives {w}px: {rows:?}"
        );
        assert!(
            rows.iter().any(|l| l.contains("0.22.22")),
            "and so does the version it lands on: {rows:?}"
        );
    }
}
