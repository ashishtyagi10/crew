//! The glass themes over more of the app than `glass_window_shot`'s two
//! file panels and a dashboard: every pane kind that opens without a shell
//! or a network, each window shot as sheer as the app ships it (the alpha is
//! the desktop showing through). Composite over a dark and a light desktop
//! to judge it — the glass's worst cases are those two.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew glass_survey_shot -- --ignored`
use crew_theme::ThemeId;

/// Pane sets, one window each: name and the commands that open its panes.
const SETS: &[(&str, &[&str])] = &[
    ("todo-keys", &["/todo", "/keys"]),
    ("usage-settings", &["/usage", "/settings"]),
    ("view-dash", &["/view Cargo.toml", "/dash"]),
    ("welcome", &[]),
];

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn glass_survey_shot() {
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::GlassClear, ThemeId::GlassNight] {
        let sheer = crate::tubesheer::sheer(1.0, id.theme());
        for (set, cmds) in SETS {
            let name = format!("survey-{}-{set}", id.as_str());
            let shot = crate::crtglassshot_tests::window_with(
                &name,
                id,
                Default::default(),
                sheer,
                cmds,
                // The bar's legend is crew's directory, as in the app.
                &|app| app.input.cwd = "/Users/you/code/crew".into(),
            );
            if shot.is_none() {
                eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
                return;
            }
        }
        // The command palette open over a pane, and a toast and an alert.
        let palette = |app: &mut crate::app::CrewApp| {
            app.input.focused = true;
            app.input.text = "/th".into();
            let now = crate::anim::now_ms();
            app.toasts
                .push("swarm finished · 3 tasks".into(), "done", false, now);
            app.toasts
                .push("cargo test failed".into(), "bell", true, now);
        };
        let name = format!("survey-{}-palette-toast", id.as_str());
        let dash: &[&str] = &["/dash"];
        crate::crtglassshot_tests::window_with(
            &name,
            id,
            Default::default(),
            sheer,
            dash,
            &palette,
        );
    }
}

/// Every glyph the glass windows draw on the glass itself (no fill of its
/// own), ranked by the worst contrast its colour makes over the pane's
/// grounds — the frost over a black and a white desktop. Prints the colours
/// under the UI floor with what they spelled: the survey's to-do list.
///
/// `cargo test -p crew-app --bin crew glass_ink_survey -- --ignored --nocapture`
#[test]
#[ignore = "a survey: prints, asserts nothing"]
fn glass_ink_survey() {
    use std::collections::BTreeMap;
    let _g = crate::app::theme_test_guard();
    let mut sets: Vec<(&str, &[&str])> = SETS.to_vec();
    sets.push(("far-dash", &["/far", "/far", "/dash"]));
    for id in [ThemeId::GlassClear, ThemeId::GlassNight] {
        crew_theme::set_theme(id);
        crew_theme::glassborder::set_sheer(true);
        let t = crew_theme::theme();
        crate::palette::set_accent(t.accent_default);
        let grounds = crew_theme::glasslegend::pane_grounds(t);
        let mut found: BTreeMap<(u8, u8, u8), Vec<String>> = BTreeMap::new();
        // The welcome's rain is decoration, faint on purpose.
        for (_, cmds) in sets.iter().filter(|(name, _)| *name != "welcome") {
            let mut app = crate::app::CrewApp {
                geo_override: Some((8.0, 18.0, 1280.0, 720.0, 1.0)),
                ..Default::default()
            };
            for cmd in *cmds {
                app.submit_input(cmd.to_string());
            }
            app.focused = 1;
            // The palette open, as the survey's last window has it.
            if cmds.len() == 3 {
                app.input.focused = true;
                app.input.text = "/th".into();
            }
            app.build_frame();
            std::thread::sleep(std::time::Duration::from_millis(400));
            // Overlays (the palette, toasts) stand on an opaque backdrop of
            // their own, not on the glass.
            for scene in app.build_frame().into_iter().filter(|s| !s.overlay) {
                let mut run = (u16::MAX, (0, 0, 0), String::new());
                for c in &scene.cells {
                    let on_glass = c.bg == t.page_bg || c.bg == t.term_bg;
                    // Frames and fills are strokes, not words: their own
                    // floor is the frame's (`glassborder`).
                    let stroke = ('\u{2500}'..='\u{259F}').contains(&c.c);
                    if c.c == ' ' || stroke || !on_glass {
                        continue;
                    }
                    if (c.row, c.fg) != (run.0, run.1) {
                        let words = std::mem::take(&mut run.2);
                        if !words.is_empty() {
                            found.entry(run.1).or_default().push(words);
                        }
                        run = (c.row, c.fg, String::new());
                    }
                    run.2.push(c.c);
                }
                if !run.2.is_empty() {
                    found.entry(run.1).or_default().push(run.2);
                }
            }
        }
        crew_theme::glassborder::set_sheer(false);
        let mut rows: Vec<_> = found
            .into_iter()
            .map(|(fg, words)| (crew_theme::glasslegend::worst(fg, &grounds), fg, words))
            .filter(|(r, _, _)| *r < crew_theme::readable::MARK_FLOOR)
            .collect();
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        eprintln!("== {}", id.as_str());
        for (r, fg, words) in rows {
            let mut w: Vec<_> = words.into_iter().filter(|s| !s.trim().is_empty()).collect();
            w.dedup();
            eprintln!("{r:5.2} {fg:?} ×{} {:?}", w.len(), &w[..w.len().min(6)]);
        }
    }
}
