//! Each theme's preferred monospace families, most-wanted first.
//!
//! A theme cannot name ONE font: fonts are per-machine. A miss makes fontdb
//! substitute a proportional face, and cell rounding then mangles every glyph
//! — so a theme states an ordered preference and the app takes the first
//! family that is actually installed (`crew-app`'s `theme_font`), changing
//! nothing when none of them are. Pure data: resolving needs the renderer's
//! installed-family list, which lives in `crew-app`, not here.
//!
//! Every list LEADS with a face that suits that theme's character — the
//! leads deliberately differ across themes, so changing themes usually
//! changes the font too (a shared lead would pin every theme to the same
//! face on any machine that has it installed). Every list ENDS in
//! [`EMBEDDED_FAMILY`], which crew ships inside its own binary, so resolution
//! cannot come up empty on any machine.
//!
//! What sits between those two is only ever a *designer* pick. The generic
//! last-resorts used to live at the tail — `Noto Sans Mono` above all, plus
//! `DejaVu Sans Mono` and `Cascadia Mono` — and they caused two separate
//! Windows bugs. First they were the only tail, and none of them exist on a
//! stock Windows box, so nothing resolved and the app drew in proportional
//! Segoe UI. Then, once the embedded face was added *behind* them, a machine
//! that happened to have one of them still preferred it — and crew was back to
//! rendering in whatever that machine's copy turned out to be. A generic
//! fallback whose whole job is "resolve something" has no job left once crew
//! ships a face of its own, so they are gone from the lists. They remain in
//! [`FONT_ALLOWLIST`], so `/font` and the rotation can still reach them; they
//! are simply never crew's automatic answer any more.
//!
//! The dated faces are deliberately not listed at all — `Cascadia Mono`
//! replaced `Consolas` on Windows the way `SF Mono` replaced `Monaco` on
//! macOS; crew prefers the modern one in both cases.
use crate::ThemeId;

/// The family crew **embeds in its own binary** (`crew-render`'s `embedfont`),
/// registered there as the monospace default. It is therefore present on every
/// machine, which is what lets every preference list below end in a face that
/// really does resolve — the previous tails (`Menlo`, `SF Mono`, `Noto Sans
/// Mono`) are stock on macOS and Linux and absent on Windows, so a fresh
/// Windows install resolved nothing and rendered in proportional Segoe UI.
///
/// Named here rather than in `crew-render` because `crew-theme` is the lower
/// crate: the preference lists and their tests need it, and a renderer
/// dependency would invert the layering.
pub const EMBEDDED_FAMILY: &str = "Lilex";

/// The only monospace families crew will *auto*-select — both theme
/// resolution (`font_prefs` below) and the `/font` rotation draw from this
/// set, intersected with what's actually installed (see `crew-app`'s
/// `font_pool`). It lists canonical names *and* the Nerd Font / installed
/// variants people really have (`ComicMono Nerd Font Mono`, `JetBrainsMono
/// NF`, …) so the intersection matches whichever spelling is present.
///
/// Deliberately excludes typewriter/legacy faces (Courier, Courier New, PT
/// Mono, Andale, Consolas, and pre-Retina Monaco — SF Mono is the modern
/// macOS face) and Stelo (its lowercase `l` renders as a broken
/// bar — user bug report 2026-07-24): a rotation must never land on one. The *manual*
/// `/font` picker is unaffected — it still offers every installed coding
/// face; this only governs what crew picks on its own. Menlo and the other
/// OS-stock faces stay ONLY as mid-list options — never a lead, and no longer
/// the safety net either ([`EMBEDDED_FAMILY`] is).
pub const FONT_ALLOWLIST: &[&str] = &[
    "JetBrains Mono",
    "JetBrainsMono NF",
    "JetBrainsMono Nerd Font",
    "Menlo",
    "Berkeley Mono",
    "Cascadia Code",
    "Cascadia Mono",
    "Comic Mono",
    "ComicMono Nerd Font Mono",
    "ComicMono Nerd Font",
    "Commit Mono",
    "CommitMono Nerd Font",
    "Fira Code",
    "FiraCode Nerd Font",
    "FiraCode Nerd Font Mono",
    "Geist Mono",
    "GeistMono Nerd Font",
    "Google Sans Code",
    "IBM Plex Mono",
    "Intel One Mono",
    "IntoneMono Nerd Font",
    "IntoneMono Nerd Font Mono",
    "Lilex",
    "Lilex Nerd Font",
    "Martian Mono",
    "MonoLisa",
    "Noto Sans Mono",
    "Operator Mono",
    "Roboto Mono",
    "RobotoMono Nerd Font",
    "RobotoMono Nerd Font Mono",
    "SF Mono",
];

/// The families this theme would like, best first. Empty = no opinion.
///
/// Each list leads with a DISTINCT theme-appropriate pick — a universal lead
/// (the old `Comic Mono` prefix) meant every theme resolved to the same face
/// wherever it was installed, so a theme rotation changed the palette but
/// never the font. Warm/paper themes keep `Comic Mono` as a mid-list option;
/// every list ends in [`EMBEDDED_FAMILY`], the face crew ships with itself, so
/// a bare machine still resolves something — and nothing generic sits in front
/// of it (see the module docs). Every entry is in [`FONT_ALLOWLIST`].
pub fn font_prefs(id: ThemeId) -> &'static [&'static str] {
    match id {
        // Paper: a book face — humanist, generous counters.
        ThemeId::PaperDark | ThemeId::PaperLight => &[
            "MonoLisa",
            "IBM Plex Mono",
            "Comic Mono",
            "ComicMono Nerd Font Mono",
            "SF Mono",
            "Menlo",
            "Lilex",
        ],
        // Sepia: warm and typewritten — friendly rounded shapes suit it, so
        // this is where the Comic Mono lead lives on.
        ThemeId::SepiaDark | ThemeId::SepiaLight => &[
            "Comic Mono",
            "ComicMono Nerd Font Mono",
            "IBM Plex Mono",
            "MonoLisa",
            "SF Mono",
            "Menlo",
            "Lilex",
        ],
        // CRT: a terminal face with squared-off shoulders — straight modern
        // faces (the old `Monaco` lead was a pre-Retina relic; Lilex is the
        // contemporary take on that IBM-terminal DNA).
        // Modern (aurora/nebula): the Gemini look wants Google's own coding
        // face; Geist is the contemporary geometric fallback.
        // The light twins share their dark parents' faces — a palette flip
        // must not also change the typeface under the user.
        // Harbor and Fern share the modern family's list: they are the same
        // kind of page, cooled.
        ThemeId::Nebula | ThemeId::Blossom | ThemeId::Harbor | ThemeId::Fern => &[
            "Google Sans Code",
            "Geist Mono",
            "GeistMono Nerd Font",
            "JetBrains Mono",
            "SF Mono",
            "Menlo",
            "Lilex",
        ],
        ThemeId::CrtGreen | ThemeId::CrtAmber | ThemeId::CrtBlue | ThemeId::CrtViolet => &[
            // The Nerd Font variant first: crew now *embeds* plain Lilex, so
            // leading with it would mean the built-in copy always beat an
            // installed icon-bearing one.
            "Lilex Nerd Font",
            "Lilex",
        ],
    }
}

#[cfg(test)]
mod tests {
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
}
