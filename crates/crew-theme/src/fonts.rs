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
//! face on any machine that has it installed). Lists end in faces that ship
//! with the OS (`Menlo`/`SF Mono` on macOS, `Noto Sans Mono`/`DejaVu Sans
//! Mono` on Linux, `Cascadia Mono` on Windows 11) so a bare machine still
//! resolves something rather than silently opting out. The dated faces are
//! deliberately not listed — `Cascadia Mono` replaced `Consolas` on Windows
//! the way `SF Mono` replaced `Monaco` on macOS; crew prefers the modern one
//! in both cases.
use crate::ThemeId;

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
/// face; this only governs what crew picks on its own. Menlo stays ONLY as
/// the OS-stock safety net at the tail of every list — never a lead.
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
/// every list ends in an OS-stock face (`Menlo`/`SF Mono`/`Noto Sans Mono`)
/// so a bare machine still resolves something. Every entry is in
/// [`FONT_ALLOWLIST`].
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
            "Noto Sans Mono",
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
            "Noto Sans Mono",
        ],
        // Midnight ink: high-contrast, tight.
        ThemeId::MidnightInk => &[
            "JetBrainsMono NF",
            "JetBrains Mono",
            "Geist Mono",
            "SF Mono",
            "Menlo",
            "Noto Sans Mono",
        ],
        // Graphite: the system's own neutral.
        ThemeId::Graphite => &[
            "SF Mono",
            "Geist Mono",
            "JetBrainsMono NF",
            "JetBrains Mono",
            "Menlo",
            "Noto Sans Mono",
        ],
        // Coldpress / glacier: flat, drafting-table — geometric and even
        // (glacier-bond is the cold blue-cast cousin of coldpress-gray).
        ThemeId::ColdpressGray | ThemeId::GlacierBond => &[
            "FiraCode Nerd Font Mono",
            "Fira Code",
            "Google Sans Code",
            "Geist Mono",
            "SF Mono",
            "Menlo",
            "Noto Sans Mono",
        ],
        // Broadsheet / ledger: newsprint and accounting — a clean modern
        // humanist face, no old typewriter Courier.
        ThemeId::SalmonBroadsheet => &[
            "MonoLisa",
            "IBM Plex Mono",
            "Comic Mono",
            "ComicMono Nerd Font Mono",
            "Menlo",
            "Noto Sans Mono",
        ],
        // (moss-blotter shares the ledger's study-desk character.)
        ThemeId::IvoryLedger | ThemeId::MossBlotter => &[
            "IBM Plex Mono",
            "SF Mono",
            "Comic Mono",
            "ComicMono Nerd Font Mono",
            "Menlo",
            "Noto Sans Mono",
        ],
        // CRT: a terminal face with squared-off shoulders — straight modern
        // faces (the old `Monaco` lead was a pre-Retina relic; Lilex is the
        // contemporary take on that IBM-terminal DNA).
        // Modern (aurora/nebula): the Gemini look wants Google's own coding
        // face; Geist is the contemporary geometric fallback.
        // The light twins share their dark parents' faces — a palette flip
        // must not also change the typeface under the user.
        ThemeId::Aurora | ThemeId::Nebula | ThemeId::Daybreak | ThemeId::Blossom => &[
            "Google Sans Code",
            "Geist Mono",
            "GeistMono Nerd Font",
            "JetBrains Mono",
            "SF Mono",
            "Menlo",
            "Noto Sans Mono",
        ],
        // Graphene: neutral and product-grade — Commit Mono's whole thesis.
        ThemeId::Graphene | ThemeId::Meadow => &[
            "Commit Mono",
            "CommitMono Nerd Font",
            "Geist Mono",
            "SF Mono",
            "Menlo",
            "Noto Sans Mono",
        ],
        // Cobalt: wide, techy, electric.
        ThemeId::Cobalt | ThemeId::Cirrus => &[
            "Martian Mono",
            "Geist Mono",
            "JetBrainsMono NF",
            "JetBrains Mono",
            "SF Mono",
            "Menlo",
            "Noto Sans Mono",
        ],
        ThemeId::CrtGreen
        | ThemeId::CrtAmber
        | ThemeId::CrtBlue
        | ThemeId::CrtViolet
        | ThemeId::CrtPaperwhite => &[
            "Lilex",
            "Lilex Nerd Font",
            "JetBrainsMono NF",
            "JetBrains Mono",
            "Google Sans Code",
            "Cascadia Mono",
            "SF Mono",
            "Menlo",
            "Noto Sans Mono",
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

    #[test]
    fn every_list_ends_in_faces_that_ship_with_an_os() {
        // Without an OS face last, a machine with none of the designer picks
        // resolves nothing and the theme silently has no font at all.
        const STOCK: [&str; 5] = [
            "Menlo",
            "SF Mono",
            "Noto Sans Mono",
            "DejaVu Sans Mono",
            "Cascadia Mono",
        ];
        for id in ALL_THEMES {
            let prefs = font_prefs(id);
            assert!(
                prefs.iter().any(|f| STOCK.contains(f)),
                "{id:?} lists only third-party faces {prefs:?} — a bare \
                 machine would resolve none of them"
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
        let mut leads: Vec<&str> = ALL_THEMES.map(|id| font_prefs(id)[0]).to_vec();
        leads.sort_unstable();
        leads.dedup();
        assert!(
            leads.len() >= 5,
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
                !["Menlo", "Noto Sans Mono", "DejaVu Sans Mono"].contains(&lead),
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
