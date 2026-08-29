//! Off-screen render of the toast stack — the notification card that docks at
//! the top-right of the content area.
//!
//! Toasts are the one surface nobody can hold still to inspect: each card is
//! on screen for 4.8 seconds and then gone. Everything about them was
//! therefore only ever asserted on — the legend's `×N` repeat count, the
//! alert stroke, the hover affordance, the clip of a message too long for the
//! card, and four of them stacked. This shoots the whole stack at once.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew toast_shot -- --ignored`
use crate::layout::Rect;
use crate::toast::Toasts;

const W: u32 = 900;
const H: u32 = 420;

/// Push a stack and shoot it exactly as `build_frame` draws it, with `cursor`
/// resting where the caller says (the hover affordance is the only state a
/// still frame cannot otherwise reach).
fn toast_shot(
    name: &str,
    cards: &[(&str, &'static str, bool, Option<&str>, usize)],
    cursor: Option<(f32, f32)>,
) -> Option<Vec<u8>> {
    let px = crate::shotdraw_tests::draw(W, H, 13.0, |cw, ch| {
        let mut toasts = Toasts::default();
        for (text, legend, alert, pane, repeats) in cards {
            for _ in 0..*repeats {
                toasts.push_for(
                    (*text).into(),
                    legend,
                    *alert,
                    1_000,
                    pane.map(str::to_string),
                );
            }
        }
        let content = Rect {
            x: 0.0,
            y: 0.0,
            w: W as f32,
            h: H as f32,
        };
        let mut scenes = Vec::new();
        // `now` a little past birth: the slide has landed, nothing is exiting.
        crate::toast::push_toasts(&mut scenes, &mut toasts, content, cw, ch, 1_400, cursor);
        scenes
    })?;
    crate::shotdraw_tests::write_png(name, &px, W, H);
    Some(px)
}

fn stack() -> Vec<(
    &'static str,
    &'static str,
    bool,
    Option<&'static str>,
    usize,
)> {
    vec![
        (
            "claude finished the diff review",
            "done",
            false,
            Some("claude"),
            1,
        ),
        (
            "agent-7 is waiting for an answer",
            "waiting",
            true,
            Some("agent-7"),
            1,
        ),
        ("build failed: 3 errors in crew-app", "error", true, None, 3),
        (
            "watch matched: TODO(perf) in crates/crew-render/src/cellgrid.rs line 412",
            "watch",
            false,
            Some("zsh"),
            1,
        ),
    ]
}

/// A full stack: a plain card, two alerts, a repeat count, and a message long
/// enough to clip. Four cards is `MAX_SHOWN` — the most anyone ever sees.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn toast_shot_full_stack() {
    let _g = crate::app::theme_test_guard();
    let Some(px) = toast_shot("toast-stack", &stack(), None) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    assert!(crate::shotgpu_tests::ink(&px) > 2000, "the stack drew");
}

/// One card with the pointer resting on it: the stroke lights and the legend
/// admits the card can be clicked. A click target with no affordance is a
/// secret.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn toast_shot_hovered() {
    let _g = crate::app::theme_test_guard();
    let one = vec![(
        "claude finished the diff review",
        "done",
        false,
        Some("claude"),
        1,
    )];
    for (name, cursor) in [
        ("toast-rest", None),
        ("toast-hover", Some((W as f32 - 120.0, 28.0))),
    ] {
        let Some(px) = toast_shot(name, &one, cursor) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 300, "{name} drew");
    }
}

/// The stack on a light page and on a tube — the alert stroke is a different
/// ramp on each.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn toast_shot_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (name, id) in [
        ("toast-light", crew_theme::ThemeId::PaperLight),
        ("toast-crt-green", crew_theme::ThemeId::CrtGreen),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let Some(px) = toast_shot(name, &stack(), None) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 2000, "{name} drew");
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

/// Print a card's three rows as text — faster than squinting at a PNG.
#[test]
#[ignore = "diagnostic"]
fn toast_dump_rows() {
    let _g = crate::app::theme_test_guard();
    let mut toasts = Toasts::default();
    toasts.push_for(
        "claude finished the diff review".into(),
        "done",
        false,
        1_000,
        Some("claude".into()),
    );
    let content = Rect {
        x: 0.0,
        y: 0.0,
        w: 900.0,
        h: 420.0,
    };
    let mut scenes = Vec::new();
    crate::toast::push_toasts(&mut scenes, &mut toasts, content, 8.0, 18.0, 1_400, None);
    for s in &scenes {
        let cols = s.cells.iter().map(|c| c.col).max().unwrap_or(0) + 1;
        for r in 0..3u16 {
            let mut line = vec![' '; cols as usize];
            for c in s.cells.iter().filter(|c| c.row == r) {
                line[c.col as usize] = c.c;
            }
            println!("{r}|{}|", line.into_iter().collect::<String>());
        }
    }
}

/// A toast is the one surface whose job is to catch your eye, and it is drawn
/// on twelve different pages. Measured rather than eyeballed: the legend that
/// names the event and the ink that carries it must read as text on every
/// one, and the bell stroke an alert wears must read as a signal.
///
/// (The shots above are what caught the eye; this is what settles the
/// argument. On the light pages the ordinary card *looks* faint next to an
/// alert — it measures 5.9:1, which is the hierarchy working, not a defect.)
#[test]
fn every_page_carries_the_toast() {
    let _g = crate::app::theme_test_guard();
    for id in crew_theme::ALL_THEMES.iter() {
        crew_theme::set_theme(*id);
        let t = crew_theme::theme();
        let r = |c| crew_theme::contrast_ratio(c, t.page_bg);
        assert!(
            r(t.legend_off) >= 4.5,
            "{id:?}: the legend naming the event reads at {:.2}",
            r(t.legend_off)
        );
        assert!(
            r(t.ink) >= 7.0,
            "{id:?}: the card's text reads at {:.2}",
            r(t.ink)
        );
        assert!(
            r(t.bell) >= 3.0,
            "{id:?}: an alert's stroke reads at {:.2}",
            r(t.bell)
        );
    }
}
