//! The terminal find bar as a window, dark and light: matches tinted, the
//! current one stronger, the bar over the top row.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew term_find_shot -- --ignored`
use crate::pane::PaneContent;
use crew_term::TermModel;
use crew_theme::ThemeId;

/// A build's output: matches in plain text, in colour, and in a line that
/// wears its own background (the ink has to be re-floored against the tint).
const TEXT: &str = "\x1b[1;32m   Compiling\x1b[0m crew-app v0.30.30 (/Users/you/code/crew)\r\n\
     \x1b[1;33mwarning\x1b[0m: unused variable: `cols`\r\n\
     \x1b[1;31merror[E0433]\x1b[0m: cannot find `table` in `md`\r\n\
     \x1b[2m   --> crates/crew-app/src/md/fold.rs:25:28\x1b[0m\r\n\
     \x1b[1;31merror[E0599]\x1b[0m: no method named `fold` found\r\n\
     \x1b[41;97m error: could not compile `crew-app` (bin \"crew\") \x1b[0m\r\n\
     Error: 2 previous errors; 1 warning emitted\r\n\
     \x1b[32m   Finished\x1b[0m test profile in crew 4.2s\r\n\
     test result: ok. 4048 passed; 0 failed; 133 ignored\r\n\
     test result: ok. 612 passed; 0 failed; 0 ignored\r\n\
     \x1b[32m\u{276f}\x1b[0m cargo test -p crew-app 2>&1 | grep error\r\n";

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn term_find_shot() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (id, query) in [
        (ThemeId::PaperDark, "error"),
        (ThemeId::PaperLight, "error"),
        (ThemeId::GlassNight, "passed"),
        (ThemeId::Porcelain, "crew"),
        (ThemeId::PaperDark, "zzzz"),
    ] {
        let prep = |app: &mut crate::app::CrewApp| {
            app.panes.push(crate::glassworkshot_tests::shell());
            app.focused = 0;
            if let PaneContent::Terminal(t) = &mut app.panes[0].content {
                // Older output above the shell's own: more to step through.
                t.pty.feed(b"\x1b[2J\x1b[H");
                t.pty.feed(TEXT.as_bytes());
                let mut f = crate::termfind::TermFind::default();
                query.chars().for_each(|c| f.edit(Some(c)));
                f.refresh(&t.pty);
                f.step(true, t.pty.screen_line());
                t.find = Some(f);
            }
        };
        let name = format!("termfind-{}-{query}", id.as_str());
        let shot = crate::crtglassshot_tests::window_with(&name, id, 1.0, &[], &prep);
        if shot.is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}
