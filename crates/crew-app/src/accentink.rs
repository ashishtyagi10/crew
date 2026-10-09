//! The accent as words.
//!
//! Split from [`crate::palette`] (at its line ceiling).
use crate::palette::accent;

/// The accent as WORDS — a key's chord, a prompt's chevron, a rate's arrow
/// — rather than as the tint of a bar or a ring. On see-through glass it is
/// floored over the desktop behind the frost: as text the iPhone's blue read
/// 1.6:1 on light glass over a dark desktop (`/keys`, 2026-10-09). The
/// bars it fills keep it as it is. Elsewhere this is [`accent`].
pub fn accent_ink() -> (u8, u8, u8) {
    let t = crew_theme::theme();
    crew_theme::glasslegend::on_glass(t, accent(), crew_theme::readable::MARK_FLOOR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crew_theme::glasslegend::{pane_grounds, worst};

    /// On glass the accent as words reads over any desktop; the bars keep
    /// the palette's accent; off glass the two are one colour.
    #[test]
    fn the_accent_reads_as_words_on_glass() {
        let _a = crate::palette::test_guard();
        let _g = crate::app::theme_test_guard();
        assert_eq!(accent_ink(), accent(), "off glass");
        for id in [
            crew_theme::ThemeId::GlassSky,
            crew_theme::ThemeId::GlassNight,
        ] {
            crew_theme::set_theme(id);
            crew_theme::glassborder::set_sheer(true);
            crate::palette::set_accent(crew_theme::theme().accent_default);
            let g = pane_grounds(crew_theme::theme());
            let got = worst(accent_ink(), &g);
            crew_theme::glassborder::set_sheer(false);
            assert!(
                got >= crew_theme::readable::MARK_FLOOR - 0.05,
                "{id:?}: {got:.2}"
            );
        }
        crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
    }
}
