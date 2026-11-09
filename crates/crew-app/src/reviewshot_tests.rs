//! A `/review` turn as the pane draws it: the reviewer's findings, each a
//! severity chip, its citation and why, and the tally in the card's corner
//! (`chatfinding`) — on a light page, a dark one, glass at night and prism.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew review_shot -- --ignored`
use crate::chatlayout::Message;
use crate::chatshot_tests::live_pane;
use crate::shotgpu_tests::shot_at;

/// A review as `reviewfindings::tidy` leaves it: bullets worst-first, then
/// the recounted verdict.
pub(crate) const REVIEW: &str = "- blocker — crates/crew-theme/src/lib.rs:42 — `luminance` swaps r and b, so every derived ink is wrong on a warm page\n- warn — src/parse.rs:118 — `unwrap()` on user input panics on an empty line\n- nit — README.md:7 — the example still names the old `--fast` flag\n\nverdict: 1 blocker, 1 warn, 1 nit";

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn review_shot() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (name, id) in [
        ("review-light", crew_theme::ThemeId::PaperLight),
        ("review-dark", crew_theme::ThemeId::PaperDark),
        ("review-glass-night", crew_theme::ThemeId::GlassNight),
        ("review-prism", crew_theme::ThemeId::Prism),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let mut p = live_pane();
        p.messages.truncate(1);
        p.messages.push(Message {
            sender: "smith → user".into(),
            text: REVIEW.into(),
            ts: String::new(),
            meta: String::new(),
            usage: None,
            expanded: false,
        });
        for w in [470, 700] {
            let shot = format!("{name}-{w}");
            if shot_at(&shot, w, 480, 13.0, "crew", |cols, rows, aspect| {
                crate::chatview::art(&p, cols, rows, aspect)
            })
            .is_none()
            {
                eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
                return;
            }
        }
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}
