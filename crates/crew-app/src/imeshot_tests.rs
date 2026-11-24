//! Off-screen shots of a composition being drawn (`imedraw`).
use super::*;

/// Off-screen: a shell mid-composition on a dark and a light theme, and the
/// input bar mid-composition on the dark one.
///
/// `CREW_SHOT_DIR=<dir> CREW_SHOT_FONT="JetBrainsMono Nerd Font Mono" \
///  cargo test -p crew-app --bin crew ime_preedit_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn ime_preedit_shot() {
    use crew_theme::ThemeId;
    use std::io::Write;
    let _g = crate::app::theme_test_guard();
    let shots = [
        (ThemeId::TokyoNight, false),
        (ThemeId::Porcelain, false),
        (ThemeId::TokyoNight, true),
    ];
    for (id, bar) in shots {
        let prep = |a: &mut CrewApp| {
            let mut p = shell();
            if let PaneContent::Terminal(t) = &mut p.content {
                t.input.write_all("echo 日本語 ".as_bytes()).unwrap();
                std::thread::sleep(std::time::Duration::from_millis(200));
                t.pty.try_read();
            }
            a.panes = vec![p];
            a.focused = 0;
            a.input.focused = bar;
            a.input.text = if bar { "echo ".into() } else { String::new() };
            a.on_ime(Ime::Preedit("にほんご".into(), Some((12, 12))));
        };
        let name = format!("ime-preedit-{id:?}{}", if bar { "-bar" } else { "" }).to_lowercase();
        let opacity = crate::tubesheer::sheer(id.theme());
        crate::crtglassshot_tests::window_with(&name, id, opacity, &[], &prep);
    }
}
