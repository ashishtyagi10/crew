//! The Ink Reader pages (the 2026-10-16 goal): `paper-light` and `paper-dark`
//! are print, an e-reader's white and black themes. What the rest of crew
//! does with a theme that has no glass, no glow and no gradient, held here
//! in one place.
use crew_theme::ThemeId::{PaperDark, PaperLight};

const INK: [crew_theme::ThemeId; 2] = [PaperLight, PaperDark];

#[test]
fn an_ink_page_is_opaque_and_flat() {
    let _g = crate::app::theme_test_guard();
    for id in INK {
        let t = id.theme();
        assert_eq!(
            crate::tubesheer::sheer(t),
            1.0,
            "{id:?}: no desktop through it"
        );
        assert!(
            t.crt.is_none() && t.modern.is_none(),
            "{id:?}: no glow, no gradient"
        );
    }
    assert!(!PaperLight.theme().dark, "light is a white page");
    assert!(PaperDark.theme().dark);
}

/// Black on white, and white on black, as an e-reader draws them.
#[test]
fn the_words_are_ink_on_paper() {
    let _g = crate::app::theme_test_guard();
    for id in INK {
        let t = id.theme();
        let r = crew_theme::contrast_ratio(t.ink, t.page_bg);
        assert!(r >= 14.0, "{id:?}: ink at {r:.1}:1");
        let lines = crew_theme::contrast_ratio(t.border_focused, t.page_bg);
        assert!(lines >= 14.0, "{id:?}: the focused line at {lines:.1}:1");
    }
}

/// No gradient anywhere: a meter is one grey, a frame one line, and the
/// focused pane's ring is not drawn.
#[test]
fn meters_and_frames_are_one_grey() {
    let _g = crate::app::theme_test_guard();
    for id in INK {
        crew_theme::set_theme(id);
        let muted = crew_theme::theme().text_muted;
        let shade = crate::summarymeter::meter_shade;
        assert_eq!((shade(0.0), shade(1.0)), (muted, muted), "{id:?}");
        assert_eq!(crew_theme::poleshift::poles(), None, "{id:?}");
    }
}

/// The cursor bar is the e-reader's inverse highlight: the page's own colour
/// on the accent, bold (`segment::inked` outside the glass).
#[test]
fn a_selection_is_page_ink_on_the_accent() {
    let _g = crate::app::theme_test_guard();
    for id in INK {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let (ink, fill) = crate::segment::inked(crate::palette::accent());
        let t = crew_theme::theme();
        assert_eq!(fill, t.accent_default, "{id:?}: the accent as it is");
        let r = crew_theme::contrast_ratio(ink, fill);
        assert!(r >= 4.5, "{id:?}: {r:.1}:1 on the bar");
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

/// Where you are in a buffer is marked in a grey of the ink, as the meters
/// are: the thumb fell back to the amber status colour, a lit mark on print.
#[test]
fn the_scroll_thumb_is_a_grey_of_the_ink() {
    let _g = crate::app::theme_test_guard();
    for id in INK {
        crew_theme::set_theme(id);
        let muted = crew_theme::theme().text_muted;
        for at in [0.0, 0.5, 1.0] {
            assert_eq!(crate::panescroll::position_fg(at), muted, "{id:?} at {at}");
        }
    }
}

/// An Ink Reader page draws no gradient, so a `/gradient` typed on one is
/// kept for the glass themes, and the note says so instead of reporting
/// colours nothing on the page shows.
#[test]
fn on_a_print_page_the_gradient_waits_for_the_glass() {
    let _g = crate::app::theme_test_guard();
    let (l0, c0) = (crate::gradientlvl::level(), crew_theme::poleshift::custom());
    let mut app = crate::app::CrewApp::default();
    crew_theme::set_theme(crew_theme::ThemeId::PaperLight);
    app.gradient_command("aurora");
    assert_eq!(app.config.gradient_poles.as_deref(), Some("aurora"));
    let said = app.active_status().unwrap_or("").to_string();
    assert!(said.contains("kept for the glass themes"), "{said:?}");
    crew_theme::set_theme(crew_theme::ThemeId::Nebula);
    app.gradient_command("reset");
    let said = app.active_status().unwrap_or("");
    assert!(!said.contains("kept for"), "glass draws it: {said:?}");
    crate::gradientlvl::set_level(l0);
    crew_theme::poleshift::set_custom(c0);
}

#[test]
fn dark_and_light_hold_their_page_and_glass_rotates_the_modern_palettes() {
    // As the user meets it (2026-10-16): `dark` and `light` are each one Ink
    // Reader page and stay on it, and the Gemini/Codex palettes come up in
    // the `glass` rotation alongside the clear panes.
    let _g = crate::app::theme_test_guard();
    let mut app = crate::app::CrewApp::default();
    // `set_theme_cmd` stamps the rotation clock with the WALL clock, so the
    // ticks have to start from there — counting up from zero is in the past
    // and `tick_random` (rightly) never fires.
    let base = crate::chattime::unix_now_ms();
    let tick = |n: u64| crew_theme::tick_random(base + n * (crew_theme::ROTATE_MS + 1_000));
    use crew_theme::ThemeId::{PaperDark, PaperLight};
    for (name, page) in [("dark", PaperDark), ("light", PaperLight)] {
        app.set_theme_cmd(name);
        for n in 1..=4u64 {
            assert!(!tick(n), "{name} repainted its one page");
            assert_eq!(crew_theme::current_id(), page, "{name} left its page");
        }
    }
    app.set_theme_cmd("glass");
    let mut seen = vec![crew_theme::current_id()];
    for n in 5..=44u64 {
        // A second past each 10-minute mark (stamped just after `base`).
        assert!(tick(n), "the rotation clock never advanced");
        let id = crew_theme::current_id();
        assert_eq!(
            id.mode(),
            crew_theme::RandomMode::Glass,
            "glass rotated off"
        );
        assert!(!id.is_crt(), "glass rotated onto a tube: {}", id.as_str());
        seen.push(id);
    }
    for id in [crew_theme::ThemeId::Nebula, crew_theme::ThemeId::Blossom] {
        assert!(
            seen.contains(&id),
            "{} never came up in `glass`",
            id.as_str()
        );
    }
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        0,
    );
}
