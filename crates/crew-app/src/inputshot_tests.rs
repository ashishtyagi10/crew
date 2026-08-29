//! Off-screen render of the docked input bar — the one row of crew that is on
//! screen in every session, and the only surface that had never been looked
//! at whole.
//!
//! The bar draws its OWN fieldset card (cwd on the top border, focused pane on
//! the bottom), so it is shot through `shotdraw_tests` directly rather than
//! nested inside the harness's card: the frame IS part of what is being
//! judged. Six states share those three rows — placeholder, ghost completion,
//! broadcast, focus mode, a status flash, an overflowing line — and each one
//! writes into the same columns the others do.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew input_shot -- --ignored`
use crew_render::PaneScene;

use crate::inputbar::InputBar;

const PAD: f32 = 12.0;

fn bar(text: &str, cwd: &str) -> InputBar {
    InputBar {
        text: text.into(),
        focused: true,
        cwd: cwd.into(),
        ..Default::default()
    }
}

/// Shoot the bar at `w` pixels wide, in the 3-row card the app docks it as.
fn input_shot(
    name: &str,
    w: u32,
    b: &InputBar,
    pending: Option<&str>,
    status: Option<&str>,
    pane: Option<&str>,
) -> Option<Vec<u8>> {
    // Cell height is only known once the grid exists, so the canvas is sized
    // for the tallest plausible cell and the card is drawn at its real height.
    let h = 3 * 24 + 2 * PAD as u32;
    let px = crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| {
        let bh = 3.0 * ch;
        let bw = w as f32 - 2.0 * PAD;
        let cols = (bw / cw).floor() as u16;
        vec![PaneScene {
            cells: b.cells(cols, 3, pending, status, pane),
            x: PAD,
            y: (h as f32 - bh) / 2.0,
            w: bw,
            h: bh,
            focused: b.focused,
            bordered: false,
            glass: true,
            scan: -1.0,
            overlay: false,
            paint: Vec::new(),
        }]
    })?;
    crate::shotdraw_tests::write_png(name, &px, w, h);
    Some(px)
}

/// One shot's inputs: the name to write, the bar itself, and the three
/// things that can claim the bottom rule.
type Case = (
    &'static str,
    InputBar,
    Option<&'static str>,
    Option<&'static str>,
    Option<&'static str>,
);

/// The states that share the bar's three rows, at the width a full window
/// gives it. A state that only ever draws in the same columns as another is a
/// state one of them silently wins.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn input_shot_states() {
    let _g = crate::app::theme_test_guard();
    let cases: Vec<Case> = vec![
        (
            "input-empty",
            bar("", "/Users/me/code/crew"),
            None,
            None,
            Some("zsh"),
        ),
        (
            "input-typing",
            bar("/das", "/Users/me/code/crew"),
            None,
            None,
            Some("zsh"),
        ),
        (
            "input-broadcast",
            InputBar {
                broadcast: true,
                ..bar("git status", "/Users/me/code/crew")
            },
            None,
            None,
            Some("claude"),
        ),
        (
            "input-status",
            bar("/theme light", "/Users/me/code/crew"),
            None,
            Some("theme \u{2192} paper light"),
            Some("zsh"),
        ),
        (
            "input-unfocused",
            InputBar {
                focused: false,
                ..bar("cargo test --workspace", "/Users/me/code/crew")
            },
            None,
            None,
            Some("cargo test --workspace -p crew-app --bin crew"),
        ),
        (
            "input-overflow",
            bar(
                "rg --hidden --glob '!target' 'fn build_frame' crates/crew-app/src | head -40",
                "/Users/me/code/crew/crates/crew-app/src/settingspane",
            ),
            None,
            None,
            Some("zsh"),
        ),
        // Browsing history: where you are, and the prefix filtering it — the
        // right end of the top rule, which the bar never used before.
        (
            "input-history",
            InputBar {
                text: "git push --force-with-lease".into(),
                history: [
                    "git status",
                    "ls -la",
                    "git push --force-with-lease",
                    "cargo test",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
                hist_pos: Some(2),
                hist_prefix: "git".into(),
                ..bar("", "/Users/me/code/crew")
            },
            None,
            None,
            Some("zsh"),
        ),
        // A ten-second window in which running the command again closes every
        // pane. It used to be visible for three of those seconds.
        (
            "input-pending",
            bar("", "/Users/me/code/crew"),
            Some("close all 4 panes? /closeall again"),
            None,
            Some("zsh"),
        ),
    ];
    for (name, b, pending, status, pane) in &cases {
        let Some(px) = input_shot(name, 1000, b, *pending, *status, *pane) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 400, "{name} drew");
    }
}

/// The bar in a quarter-window and in a wide one. Its legend, its prompt, its
/// text and its bottom-border pane name all compete for the same row pair.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn input_shot_width_sweep() {
    let _g = crate::app::theme_test_guard();
    for (name, w) in [("input-narrow", 380), ("input-wide", 1400)] {
        let b = bar("/model claude-opus", "/Users/me/code/crew");
        let Some(px) = input_shot(name, w, &b, None, None, Some("claude")) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 200, "{name} drew");
    }
}

/// Focus mode puts a standing tag in front of the path — on a light page and
/// on a tube, where the legend's colour comes from a different ramp.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn input_shot_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (name, id) in [
        ("input-light", crew_theme::ThemeId::PaperLight),
        ("input-crt-green", crew_theme::ThemeId::CrtGreen),
        ("input-sepia", crew_theme::ThemeId::SepiaLight),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let b = bar("/gradient dusk", "/Users/me/code/crew");
        let Some(px) = input_shot(name, 1000, &b, None, None, Some("zsh")) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 400, "{name} drew");
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

/// Print the bar's three rows as text — the fastest way to see which cell a
/// legend, a label or a corner actually landed in.
#[test]
#[ignore = "diagnostic"]
fn input_dump_rows() {
    let _g = crate::app::theme_test_guard();
    let b = bar(
        "rg --hidden --glob '!target' 'fn build_frame' crates/crew-app/src | head -40",
        "/Users/me/code/crew",
    );
    let cols = 52u16;
    let cells = b.cells(
        cols,
        3,
        None,
        None,
        Some("cargo test --workspace -p crew-app --bin crew"),
    );
    for r in 0..3u16 {
        let mut line = vec![' '; cols as usize];
        for c in cells.iter().filter(|c| c.row == r) {
            if (c.col as usize) < line.len() {
                line[c.col as usize] = c.c;
            }
        }
        println!("{r}|{}|", line.into_iter().collect::<String>());
    }
}
