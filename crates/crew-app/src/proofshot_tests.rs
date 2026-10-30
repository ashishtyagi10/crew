//! The proof under a turn's receipt, with the run's own count in it: a green
//! run, a red one naming its failures, and a run of no tests — at a quarter
//! tile and a half, on a light page and on glass.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew proof_shot -- --ignored`
use crate::chatlayout::Message;
use crate::chatshot_tests::live_pane;
use crate::shotgpu_tests::shot_at;

const RECEIPT: &str =
    "task #3: 1 file changed (+4 \u{2212}1): ~ crates/crew-theme/src/lib.rs\n```diff\n-    (r, g, b)\n+    (b, g, r)\n```";

fn smith(text: &str) -> Message {
    Message {
        sender: "agent smith".into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
    }
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn proof_shot() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let verdicts = [
        "\u{2713} checked: `cargo test -p crew-theme` \u{2014} 42 tests passed in 12s after the last edit",
        "\u{2717} failing: `cargo test -p crew-theme` \u{2014} 2 of 44 tests failed after the last edit: luminance_swaps, ratio_is_symmetric",
        "! not checked: `cargo test -p crew-theme lum` ran no tests after the last edit",
    ];
    for (name, id) in [
        ("proof-light", crew_theme::ThemeId::PaperLight),
        ("proof-glass-night", crew_theme::ThemeId::GlassNight),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let mut p = live_pane();
        p.messages.truncate(1);
        for v in verdicts {
            p.messages.push(smith(RECEIPT));
            assert!(p.into_receipt("agent smith", v));
        }
        for w in [470, 700] {
            let shot = format!("{name}-{w}");
            if shot_at(&shot, w, 760, 13.0, "crew", |cols, rows, aspect| {
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
