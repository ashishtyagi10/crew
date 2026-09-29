//! The Shanns faces (the user's installs, 2026-09-27): Comic Shanns is a
//! favourite in the spellings this Mac actually has and leads the Sepia
//! family; Serious Shanns was asked out the next day and must stay out.
use crate::{font_prefs, is_favorite, typeface_key, ThemeId, ALL_THEMES, FONT_ALLOWLIST};

#[test]
fn comic_shanns_is_a_favourite_however_it_is_installed() {
    for fam in ["Comic Shanns Mono", "ComicShannsMono Nerd Font Mono"] {
        assert!(is_favorite(fam), "{fam}");
        assert!(
            FONT_ALLOWLIST
                .iter()
                .any(|f| typeface_key(f) == typeface_key(fam)),
            "{fam} is reachable by the rotation"
        );
    }
    // Its own typeface, not Comic Mono's under another name.
    assert_ne!(
        typeface_key("Comic Shanns Mono"),
        typeface_key("Comic Mono")
    );
}

#[test]
fn serious_shanns_is_out_of_every_list_in_every_spelling() {
    for fam in ["Serious Shanns", "SeriousShanns Nerd Font Mono"] {
        let key = typeface_key(fam);
        assert!(!is_favorite(fam), "{fam} is still a favourite");
        assert!(
            !FONT_ALLOWLIST.iter().any(|f| typeface_key(f) == key),
            "{fam} is still reachable by the rotation"
        );
        for id in ALL_THEMES {
            assert!(
                !font_prefs(id).iter().any(|f| typeface_key(f) == key),
                "{id:?} still prefers {fam}"
            );
        }
    }
}

#[test]
fn sepia_leads_with_comic_shanns_and_paper_with_monolisa_again() {
    assert_eq!(
        &font_prefs(ThemeId::SepiaDark)[..2],
        ["Comic Shanns Mono", "Comic Mono"]
    );
    assert_eq!(font_prefs(ThemeId::PaperLight)[0], "MonoLisa");
}
