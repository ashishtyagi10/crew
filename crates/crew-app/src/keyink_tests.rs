use super::*;
use crew_theme::{contrast_ratio, ThemeId};

fn rgb(s: Style) -> (u8, u8, u8) {
    match s.fg {
        Some(Color::Rgb(r, g, b)) => (r, g, b),
        other => panic!("not an rgb fg: {other:?}"),
    }
}

/// Runs `f` on `id` with that theme's own accent, as switching to it sets.
fn on(id: ThemeId, f: impl FnOnce(&crew_theme::Theme)) {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(id);
    let t = crew_theme::theme();
    crate::palette::set_accent(t.accent_default);
    f(t);
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

/// On the Ink pages the accent is a grey, so `/keys`' key column has no
/// hue to stand on: it goes bold, in an ink at least as strong as its
/// description's. On paper-light the key was the paler of the two.
#[test]
fn a_key_outweighs_its_label_on_the_ink_pages() {
    for id in [ThemeId::PaperLight, ThemeId::PaperDark] {
        on(id, |t| {
            let (key, label) = key_label();
            assert!(
                key.add_modifier.contains(Modifier::BOLD),
                "{id:?}: key not bold"
            );
            let k = contrast_ratio(rgb(key), t.page_bg);
            let l = contrast_ratio(rgb(label), t.page_bg);
            assert!(k >= l, "{id:?}: key {k:.2} fainter than label {l:.2}");
        });
    }
}

/// A hued accent is the key's colour, as before, and needs no weight.
#[test]
fn a_hued_accent_still_marks_the_key() {
    on(ThemeId::Nebula, |_| {
        let (key, _) = key_label();
        assert_eq!(rgb(key), accent_ink());
        assert!(!key.add_modifier.contains(Modifier::BOLD));
    });
}

/// The welcome's chords (`⌘T ⌘O /`) are drawn no fainter than the page's
/// ink on paper-light; they were the accent's grey, paler than the ink.
#[test]
fn the_welcome_chords_are_not_paler_than_ink_on_paper() {
    on(ThemeId::PaperLight, |t| {
        let c = crate::welcometext::chord_fg();
        assert!(contrast_ratio(c, t.page_bg) >= contrast_ratio(t.ink, t.page_bg));
    });
}
