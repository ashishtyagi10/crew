//! The proof under a turn's receipt, with the run's own count in it: a red
//! run turned green (`chatrepro`), a green run, a red one naming its
//! failures, and a run of no tests — at a quarter tile and a half, on a
//! light page and on glass — the last one with the plan steps its turn
//! left undone.
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
        "\u{2713} fixed: `cargo test -p crew-theme` \u{2014} 2 of 44 failed before the fix; 44 pass (12s)",
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
        // The last turn stopped short of its plan (`chatchecklist::left_note`).
        let left = "\u{25cb} plan 2/5 \u{b7} left: Run the tests, +2 more";
        assert!(p.into_receipt("agent smith", left));
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

/// A receipt of several files, drawn (`chatreceipthead`): a row each with its
/// counts and bar, in four themes (light, dark, glass at night, CRT).
///
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew receipt_head_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn receipt_head_shot() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let many = "task #7 stopped \u{b7} 5 files changed (+9 \u{2212}3): ~ crates/crew-app/src/chatmsgs.rs, + crates/crew-app/src/chatreceipthead.rs, ~ src/c.rs, ~ src/d.rs, +1 more \u{2014} /diff shows them; \u{201c}undo that\u{201d} puts them back\n```diff\ndiff --git a/crates/crew-app/src/chatmsgs.rs b/crates/crew-app/src/chatmsgs.rs\n+x\n+y\n-z\ndiff --git a/crates/crew-app/src/chatreceipthead.rs b/crates/crew-app/src/chatreceipthead.rs\n+1\n+2\n+3\n+4\n+5\n+6\ndiff --git a/src/c.rs b/src/c.rs\n-q\n-r\n```";
    for (name, id) in [
        ("receipt-light", crew_theme::ThemeId::PaperLight),
        ("receipt-dark", crew_theme::ThemeId::PaperDark),
        ("receipt-glass-night", crew_theme::ThemeId::GlassNight),
        ("receipt-crt", crew_theme::ThemeId::CrtGreen),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let mut p = live_pane();
        p.messages.truncate(1);
        p.messages.push(smith(RECEIPT));
        assert!(p.into_receipt("agent smith", "\u{2713} checked: `cargo test` passed"));
        p.messages.push(smith(many));
        for w in [470, 700] {
            let shot = format!("{name}-{w}");
            if shot_at(&shot, w, 520, 13.0, "crew", |cols, rows, aspect| {
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
