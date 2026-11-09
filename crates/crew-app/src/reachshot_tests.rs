//! A receipt that says how far its change reaches (`chatreceiptreach`) and
//! names the callers a signature change left behind — one file and two, a
//! quarter tile and a half, on a light page and on glass at night.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew reach_shot -- --ignored`
use crate::chatlayout::Message;
use crate::chatshot_tests::live_pane;
use crate::shotgpu_tests::shot_at;

const ONE: &str = "task #5: 1 file changed (+1 \u{2212}1): ~ src/lib.rs \u{b7} reaches 2 callers in 2 files\n```diff\ndiff --git a/src/lib.rs b/src/lib.rs\n-pub fn add(a: i32, b: i32) -> i32 {\n+pub fn add(a: i32, b: i32, c: i32) -> i32 {\n```";
const TWO: &str = "task #6: 2 files changed (+4 \u{2212}2): ~ src/lib.rs, ~ src/main.rs \u{b7} reaches 6 callers in 3 files\n```diff\ndiff --git a/src/lib.rs b/src/lib.rs\n-x\n+y\n+z\ndiff --git a/src/main.rs b/src/main.rs\n-p\n+q\n+r\n```";

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
fn reach_shot() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (name, id) in [
        ("reach-light", crew_theme::ThemeId::PaperLight),
        ("reach-glass-night", crew_theme::ThemeId::GlassNight),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let mut p = live_pane();
        p.messages.truncate(1);
        p.messages.push(smith(ONE));
        let stale = "! 2 callers of add() not updated: src/app.rs:4 \u{b7} src/cli.rs:2";
        assert!(p.into_receipt("agent smith", stale));
        p.messages.push(smith(TWO));
        assert!(p.into_receipt("agent smith", "\u{2713} checked: `cargo test` passed"));
        for w in [470, 700] {
            let shot = format!("{name}-{w}");
            if shot_at(&shot, w, 560, 13.0, "crew", |cols, rows, aspect| {
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
