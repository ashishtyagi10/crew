//! The accent on glass.
//!
//! Split from [`crate::palette`] (at its line ceiling).
use crate::palette::accent;

/// The accent on see-through glass, as [`accent`] serves it: floored over
/// the desktop behind the frost, words and marks alike. As words the
/// iPhone's blue read 1.6:1 on light glass over a dark desktop (`/keys`,
/// 2026-10-09), and the survey that followed found it raw in the composer's
/// `❯`, the thinking `∴`, the header's spinner, the nav's dots, the welcome's
/// chords and a reply's headings — 49 call sites, so the floor moved here,
/// under all of them. A bar it fills has to be seen over the frost too.
/// Elsewhere `c` passes through.
pub(crate) fn glassed(c: (u8, u8, u8)) -> (u8, u8, u8) {
    crew_theme::glasslegend::on_glass(crew_theme::theme(), c, crew_theme::readable::MARK_FLOOR)
}

/// The accent as WORDS — a key's chord, a prompt's chevron, a rate's arrow.
/// Since [`accent`] itself is floored on glass ([`glassed`]) this is the
/// accent; the name stays for the sites that mean words.
pub fn accent_ink() -> (u8, u8, u8) {
    glassed(accent())
}

/// The FOCUS accent on see-through glass: walked to read over the pane's
/// frost on any desktop, as [`accent_ink`] is, so focus wears the blue the
/// accent's words do, at the floor bold text needs. On light glass over a
/// dark desktop the iPhone's blue read 1.62:1 as the settings form's focused
/// legend, fainter than the muted legends around it (3.5). No lightness can
/// both read there and sit 1.6:1 from those legends (they are floored over
/// the same frost), so on glass focus is told from muted by hue: a saturated
/// blue against a grey, and in bold. Elsewhere `focus` passes through.
pub(crate) fn focus_on_glass(focus: (u8, u8, u8)) -> (u8, u8, u8) {
    match crew_theme::glassborder::sheer() {
        true => crew_theme::glasslegend::on_glass(
            crew_theme::theme(),
            focus,
            crew_theme::readable::MARK_FLOOR,
        ),
        false => focus,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crew_theme::glasslegend::{pane_grounds, worst};

    /// On glass the drawn accent — words, marks and bars — reads over any
    /// desktop; off glass it is the palette's own.
    #[test]
    fn the_accent_reads_as_words_on_glass() {
        let _a = crate::palette::test_guard();
        let _g = crate::app::theme_test_guard();
        assert_eq!(accent_ink(), accent(), "off glass");
        for id in [
            crew_theme::ThemeId::GlassClear,
            crew_theme::ThemeId::GlassSky,
            crew_theme::ThemeId::GlassNight,
        ] {
            crew_theme::set_theme(id);
            crew_theme::glassborder::set_sheer(true);
            crate::palette::set_accent(crew_theme::theme().accent_default);
            let g = pane_grounds(crew_theme::theme());
            let got = worst(accent_ink(), &g).min(worst(accent(), &g));
            crew_theme::glassborder::set_sheer(false);
            assert!(
                got >= crew_theme::readable::MARK_FLOOR - 0.05,
                "{id:?}: {got:.2}"
            );
        }
        crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
    }

    /// The settings form's focused legend reads over the frost on any
    /// desktop, and still stands apart from the muted legends it replaces —
    /// by hue where lightness has no room (a blue against a grey).
    #[test]
    fn the_focus_accent_reads_on_glass_and_apart_from_muted() {
        let _a = crate::palette::test_guard();
        let _g = crate::app::theme_test_guard();
        for id in [
            crew_theme::ThemeId::GlassClear,
            crew_theme::ThemeId::GlassSky,
            crew_theme::ThemeId::GlassDawn,
            crew_theme::ThemeId::GlassNight,
        ] {
            crew_theme::set_theme(id);
            crew_theme::glassborder::set_sheer(true);
            let t = crew_theme::theme();
            crate::palette::set_accent(t.accent_default);
            let focus = crate::palette::focus_accent();
            let got = worst(focus, &pane_grounds(t));
            let apart = crew_theme::oklch::distance(focus, t.text_muted);
            crew_theme::glassborder::set_sheer(false);
            let floor = crew_theme::readable::MARK_FLOOR;
            assert!(got >= floor - 0.05, "{id:?}: focus over the frost {got:.2}");
            // A rung of the text hierarchy (`oklch::distance`'s table).
            assert!(apart >= 0.10, "{id:?}: focus vs muted {apart:.3}");
        }
        crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
    }
}
