//! Each theme with its own default accent, as a config with no `accent` key
//! draws it (`Config::accent_rgb`).
use crew_theme::{contrast_ratio, ThemeId, ALL_THEMES};

fn wear(id: ThemeId) -> &'static crew_theme::Theme {
    crew_theme::set_theme(id);
    crate::palette::set_accent(crew_theme::theme().accent_default);
    crew_theme::theme()
}

/// The heading ladder runs the right way on every palette: the title reads at
/// least as hard as an h2 (the ink), or wears a hue that says "label" itself.
#[test]
fn the_title_never_reads_fainter_than_an_h2_without_a_hue() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for id in ALL_THEMES {
        let th = wear(id);
        let (h1, h2) = (
            crate::chatspan::heading_fg(1),
            crate::chatspan::heading_fg(2),
        );
        let hued = crew_theme::oklch::from_srgb(h1).c >= super::GREY_CHROMA;
        let (a, b) = (
            contrast_ratio(h1, th.page_bg),
            contrast_ratio(h2, th.page_bg),
        );
        assert!(hued || a >= b, "{id:?}: h1 {a:.1} under h2 {b:.1}");
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

/// The page the survey measured: h1 and column names at 7.1:1 under body
/// text at 16.3:1. They now print in the darkest ink on the page.
#[test]
fn on_the_ink_reader_light_page_the_title_is_the_darkest_ink() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let th = wear(ThemeId::PaperLight);
    let got = contrast_ratio(super::ink(), th.page_bg);
    assert!(got >= contrast_ratio(th.ink, th.page_bg), "{got:.2}");
    assert_ne!(super::ink(), crate::palette::accent());
    // The column names wear it too.
    let table =
        crate::chatbody::body_lines("| name | b |\n|---|---|\n| 1 | 2 |", 30, th.ink, false);
    let head = table[0].iter().find(|c| c.c == 'n').expect("header cell");
    assert_eq!(head.fg, super::ink());
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

/// A hued accent keeps the job: the glass and tube themes, and an Ink page
/// whose user picked a colour, are drawn as before.
#[test]
fn a_hued_accent_is_left_as_it_is() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for id in ALL_THEMES {
        if !wear(id).is_print() {
            assert_eq!(super::ink(), crate::palette::accent(), "{id:?}");
        }
    }
    wear(ThemeId::PaperLight);
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
    assert_eq!(super::ink(), crate::palette::accent(), "a picked green");
}
