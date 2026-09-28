//! The Shanns faces (the user's installs, 2026-09-27) are favourites in the
//! spellings this Mac actually has, and lead the two warm theme families.
use crate::{font_prefs, is_favorite, typeface_key, ThemeId, FONT_ALLOWLIST};

#[test]
fn the_shanns_faces_are_favourites_however_they_are_installed() {
    for fam in [
        "Comic Shanns Mono",
        "SeriousShanns Nerd Font Mono",
        "Serious Shanns",
    ] {
        assert!(is_favorite(fam), "{fam}");
        assert!(
            FONT_ALLOWLIST
                .iter()
                .any(|f| typeface_key(f) == typeface_key(fam)),
            "{fam} is reachable by the rotation"
        );
    }
    // Two typefaces, not one: the keys must not collide with Comic Mono's.
    assert_ne!(
        typeface_key("Comic Shanns Mono"),
        typeface_key("Comic Mono")
    );
    assert_ne!(
        typeface_key("Serious Shanns"),
        typeface_key("Comic Shanns Mono")
    );
}

#[test]
fn they_lead_the_warm_families_with_the_old_leads_right_behind() {
    assert_eq!(
        &font_prefs(ThemeId::SepiaDark)[..2],
        ["Comic Shanns Mono", "Comic Mono"]
    );
    assert_eq!(
        &font_prefs(ThemeId::PaperLight)[..2],
        ["Serious Shanns", "MonoLisa"]
    );
}
