use super::*;
use crate::fonts::FONT_ALLOWLIST;

#[test]
fn one_typeface_keys_the_same_however_it_was_installed() {
    for spellings in [
        &[
            "JetBrains Mono",
            "JetBrainsMono NF",
            "JetBrainsMono Nerd Font",
            "JetBrainsMono Nerd Font Mono",
        ][..],
        &[
            "Comic Mono",
            "ComicMono Nerd Font",
            "ComicMono Nerd Font Mono",
        ][..],
        &["Fira Code", "FiraCode Nerd Font", "FiraCode Nerd Font Mono"][..],
        &["Lilex", "Lilex Nerd Font"][..],
        &[
            "Roboto Mono",
            "RobotoMono Nerd Font",
            "RobotoMono Nerd Font Mono",
        ][..],
    ] {
        let keys: Vec<String> = spellings.iter().map(|s| typeface_key(s)).collect();
        assert!(
            keys.windows(2).all(|w| w[0] == w[1]),
            "{spellings:?} keyed as {keys:?}"
        );
        assert!(!keys[0].is_empty(), "{spellings:?} keyed as nothing");
    }
}

#[test]
fn two_different_faces_never_share_a_key() {
    // The whole allowlist, grouped: a collision here would make the rotation
    // treat two unrelated faces as one and never show the second.
    let mut seen: Vec<(String, &str)> = Vec::new();
    for fam in FONT_ALLOWLIST {
        let key = typeface_key(fam);
        if let Some((_, other)) = seen.iter().find(|(k, _)| *k == key) {
            // Same key is only allowed between spellings of one typeface, and
            // those always share a leading word.
            let head = |s: &str| s.to_ascii_lowercase().replace(' ', "");
            assert!(
                head(fam).starts_with(&head(other)[..4.min(head(other).len())]),
                "{fam:?} and {other:?} both key as {key:?}"
            );
        }
        seen.push((key, fam));
    }
}

#[test]
fn every_favourite_is_a_face_the_rotation_can_actually_reach() {
    // A favourite outside the allowlist is never in the pool, so weighting it
    // would be weighting something that can never come up.
    for fav in FAVORITES {
        assert!(
            FONT_ALLOWLIST
                .iter()
                .any(|f| typeface_key(f) == typeface_key(fav)),
            "{fav} is a favourite the pool can never offer"
        );
    }
}

#[test]
fn a_favourite_is_recognised_in_whatever_spelling_is_installed() {
    for fam in [
        "Comic Mono",
        "ComicMono Nerd Font Mono",
        "JetBrainsMono Nerd Font",
        "SF Mono",
        "IBM Plex Mono",
        "MonoLisa",
        "Lilex Nerd Font",
    ] {
        assert!(is_favorite(fam), "{fam} should be a favourite");
    }
    for fam in [
        "Menlo",
        "Fira Code",
        "Geist Mono",
        "Google Sans Code",
        "Roboto Mono",
    ] {
        assert!(!is_favorite(fam), "{fam} should not be a favourite");
    }
}

#[test]
fn a_favourite_can_be_weighted_without_becoming_a_theme_preference() {
    // Noto Sans Mono is a favourite by request and belongs in NO theme's
    // preference list. The rotation is a thing the user switches on; the
    // preference lists are what crew picks with nobody asking, and a generic
    // face at the front of those is both Windows bugs in the 0.17.x notes.
    // If this ever needs deleting, delete it knowing that.
    assert!(is_favorite("Noto Sans Mono"));
    for id in crate::ALL_THEMES {
        assert!(
            !crate::font_prefs(id).contains(&"Noto Sans Mono"),
            "{id:?} would auto-resolve to the generic fallback"
        );
    }
}

#[test]
fn a_nameless_family_is_not_a_favourite_by_keying_to_nothing() {
    // "Mono" alone keys as the empty string, and an empty key must not match
    // every other empty key and turn an unknown face into a favourite.
    assert_eq!(typeface_key("Mono"), "");
    assert!(!is_favorite("Mono"));
    assert!(!is_favorite(""));
}

/// The instruction behind this file, as an invariant: the face a theme picks
/// on its own is one the user asked for. Six favourites and four theme
/// families, so they cannot each lead — but nothing the user did NOT name may
/// lead, which is the part that can drift.
#[test]
fn every_theme_leads_with_a_face_the_user_asked_for() {
    for id in crate::ALL_THEMES {
        let lead = crate::font_prefs(id)[0];
        assert!(
            is_favorite(lead),
            "{id:?} leads with {lead:?}, which is not one of {FAVORITES:?}"
        );
    }
}

/// A lead per family, still: the rotation has to change the FONT as well as
/// the palette, and four favourites in four families is what does that.
#[test]
fn the_theme_families_do_not_all_lead_with_the_same_favourite() {
    let mut leads: Vec<String> = crate::ALL_THEMES
        .iter()
        .map(|id| typeface_key(crate::font_prefs(*id)[0]))
        .collect();
    leads.sort();
    leads.dedup();
    assert!(
        leads.len() >= 4,
        "only {} distinct faces lead: {leads:?}",
        leads.len()
    );
}

#[test]
fn operator_mono_is_weighted_but_never_guessed_for_you() {
    // Added by request (2026-09-16). It is a face you BUY, so on most
    // machines it is not installed — which is fine in the rotation, a thing
    // you switch on, and wrong in a preference list, which is crew choosing
    // with nobody asking.
    assert!(is_favorite("Operator Mono"));
    assert!(is_favorite("OperatorMono Nerd Font"), "any spelling counts");
    for id in crate::ALL_THEMES {
        assert!(
            !crate::fonts::font_prefs(id)
                .iter()
                .any(|f| typeface_key(f) == typeface_key("Operator Mono")),
            "{id:?} would guess Operator Mono, which most machines lack"
        );
    }
}
