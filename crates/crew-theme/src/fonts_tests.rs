use super::*;
use crate::ALL_THEMES;

#[test]
fn every_theme_states_a_preference() {
    for id in ALL_THEMES {
        assert!(
            !font_prefs(id).is_empty(),
            "{id:?} has no font preference — the match arm is missing"
        );
    }
}

/// The test this replaces accepted an OS-stock face anywhere in the list
/// — and counted `Menlo` as proof. Menlo is macOS-only, so the assertion
/// held on the dev machine while 23 of 24 themes named nothing a stock
/// Windows box has. Resolution came up empty there, shaping fell through
/// to proportional Segoe UI, and `set_monospace_width` rounded its narrow
/// advances to zero: the mangled banner and nav of a fresh 0.17.8 install.
///
/// [`EMBEDDED_FAMILY`] ships inside the binary, so this assertion means
/// the same thing on every platform — it cannot be satisfied by a face
/// that merely happens to be on the machine running the test.
#[test]
fn every_list_names_the_embedded_family_so_resolution_cannot_come_up_empty() {
    for id in ALL_THEMES {
        let prefs = font_prefs(id);
        assert!(
            prefs.contains(&EMBEDDED_FAMILY),
            "{id:?} lists {prefs:?}, none of which crew ships — on a \
             machine with none of them installed the theme resolves no \
             font at all and shaping falls back to a proportional face"
        );
    }
}

/// The second Windows bug, pinned: a *generic* fallback must never sit in
/// front of the embedded face.
///
/// Shipping Lilex fixed the machine that resolved nothing. It did not fix
/// the machine that had `Noto Sans Mono`, because that name came first in
/// every list — so crew still picked whatever the machine's copy was and
/// still drew a broken grid. These names earn a place only by being
/// somebody's deliberate choice of coding face; "it resolves" is no longer
/// a qualification.
#[test]
fn no_generic_fallback_outranks_the_embedded_family() {
    const GENERIC: [&str; 3] = ["Noto Sans Mono", "DejaVu Sans Mono", "Liberation Mono"];
    for id in ALL_THEMES {
        let prefs = font_prefs(id);
        let at = prefs.iter().position(|f| *f == EMBEDDED_FAMILY).unwrap();
        for g in GENERIC {
            assert!(
                !prefs[..at].contains(&g),
                "{id:?} prefers the generic {g:?} over the face crew ships \
                 — on a machine that has it, crew renders in it whatever \
                 it turns out to be"
            );
        }
    }
}

/// Nothing may sit *after* the embedded family: it always resolves, so a
/// later entry is unreachable and reads as an intent the app never honours.
#[test]
fn nothing_is_listed_after_the_embedded_family() {
    for id in ALL_THEMES {
        let prefs = font_prefs(id);
        let at = prefs.iter().position(|f| *f == EMBEDDED_FAMILY).unwrap();
        let dead = &prefs[at + 1..];
        assert!(
            dead.is_empty(),
            "{id:?} lists {dead:?} after {EMBEDDED_FAMILY:?}, which always \
             resolves — those entries can never be reached"
        );
    }
}

#[test]
fn every_pref_is_in_the_allowlist() {
    // A theme must never auto-resolve to a face outside the curated
    // allowlist (that is how a stray Courier/typewriter face crept in).
    for id in ALL_THEMES {
        for fam in font_prefs(id) {
            assert!(
                FONT_ALLOWLIST.contains(fam),
                "{id:?} lists {fam:?}, which is not in FONT_ALLOWLIST"
            );
        }
    }
}

#[test]
fn theme_leads_are_diverse_so_rotation_changes_the_font() {
    // The regression behind "the theme rotates but the font never
    // changes": every list led with the same Comic Mono pair, so any
    // machine with it installed resolved every theme to the same face.
    //
    // Four after the 24→9 cut, which is one per surviving family — paper,
    // sepia, modern and CRT. That is the real invariant the old ">= 5"
    // was reaching for: a family gets its own face, and the count follows
    // the roster rather than leading it.
    let mut leads: Vec<&str> = ALL_THEMES.map(|id| font_prefs(id)[0]).to_vec();
    leads.sort_unstable();
    leads.dedup();
    assert!(
        leads.len() >= 4,
        "only {} distinct lead families across all themes: {leads:?}",
        leads.len()
    );
}

#[test]
fn allowlist_has_no_typewriter_or_legacy_system_faces() {
    for banned in [
        "Courier",
        "Courier New",
        "PT Mono",
        "Andale Mono",
        "Consolas",
        "Monaco",
    ] {
        assert!(
            !FONT_ALLOWLIST.contains(&banned),
            "{banned} must not be auto-selectable"
        );
    }
}

/// Menlo ships with macOS and earns its keep as the never-fail tail of a
/// preference list, but it is a 2009 face — no theme should *lead* with
/// it (nor with any other OS-stock fallback; leads are designer picks).
#[test]
fn no_theme_leads_with_a_stock_fallback_face() {
    for id in ALL_THEMES {
        let lead = font_prefs(id)[0];
        assert!(
            ![
                "Menlo",
                "Noto Sans Mono",
                "DejaVu Sans Mono",
                EMBEDDED_FAMILY
            ]
            .contains(&lead),
            "{id:?} leads with the fallback face {lead:?}"
        );
    }
}

#[test]
fn no_list_repeats_a_family() {
    for id in ALL_THEMES {
        let prefs = font_prefs(id);
        let mut seen: Vec<&str> = prefs.to_vec();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(before, seen.len(), "{id:?} repeats a family: {prefs:?}");
    }
}
