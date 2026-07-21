//! Each theme's preferred monospace families, most-wanted first.
//!
//! A theme cannot name ONE font: fonts are per-machine. A miss makes fontdb
//! substitute a proportional face, and cell rounding then mangles every glyph
//! — so a theme states an ordered preference and the app takes the first
//! family that is actually installed (`crew-app`'s `theme_font`), changing
//! nothing when none of them are. Pure data: resolving needs the renderer's
//! installed-family list, which lives in `crew-app`, not here.
//!
//! Every list leads with `Comic Mono` (a rounded, friendly coding face) and
//! ends in faces that ship
//! with the OS (`Menlo`/`SF Mono` on macOS, `Noto Sans Mono`/`DejaVu Sans Mono`
//! on Linux, `Cascadia Mono` on Windows 11) so a bare machine still resolves
//! something rather than silently opting out. The dated `Consolas` is
//! deliberately not listed — `Cascadia Mono` is the modern Windows stock face.
use crate::ThemeId;

/// The only monospace families crew will *auto*-select — both theme
/// resolution (`font_prefs` below) and the `/font` rotation draw from this
/// set, intersected with what's actually installed (see `crew-app`'s
/// `font_pool`). It lists canonical names *and* the Nerd Font / installed
/// variants people really have (`ComicMono Nerd Font Mono`, `JetBrainsMono
/// NF`, …) so the intersection matches whichever spelling is present.
///
/// Deliberately excludes typewriter/legacy faces (Courier, Courier New, PT
/// Mono, Andale, Consolas): a rotation must never land on one. The *manual*
/// `/font` picker is unaffected — it still offers every installed coding
/// face; this only governs what crew picks on its own.
pub const FONT_ALLOWLIST: &[&str] = &[
    "JetBrains Mono",
    "JetBrainsMono NF",
    "JetBrainsMono Nerd Font",
    "Menlo",
    "Comic Mono",
    "ComicMono Nerd Font Mono",
    "ComicMono Nerd Font",
    "Fira Code",
    "FiraCode Nerd Font",
    "FiraCode Nerd Font Mono",
    "Geist Mono",
    "Google Sans Code",
    "IBM Plex Mono",
    "Lilex",
    "MonoLisa",
    "Noto Sans Mono",
    "Operator Mono",
    "Roboto Mono",
    "RobotoMono Nerd Font",
    "RobotoMono Nerd Font Mono",
    "Monaco",
    "SF Mono",
    "Stelo",
];

/// The families this theme would like, best first. Empty = no opinion.
///
/// Each list leads with `Comic Mono` and its installed `ComicMono Nerd Font
/// Mono` variant (so the friendly default resolves whether you have the base
/// face or the Nerd Font build), then theme-appropriate picks, and ends in an
/// OS-stock face (`Menlo`/`SF Mono`/`Noto Sans Mono`) so a bare machine still
/// resolves something. Every entry is in [`FONT_ALLOWLIST`].
pub fn font_prefs(id: ThemeId) -> &'static [&'static str] {
    match id {
        // Paper: a book face — humanist, generous counters.
        ThemeId::PaperDark | ThemeId::PaperLight => &[
            "Comic Mono",
            "ComicMono Nerd Font Mono",
            "MonoLisa",
            "IBM Plex Mono",
            "SF Mono",
            "Menlo",
            "Noto Sans Mono",
        ],
        // Sepia: warm and typewritten — a modern humanist face, no old Courier.
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
            "Comic Mono",
            "ComicMono Nerd Font Mono",
            "JetBrainsMono NF",
            "JetBrains Mono",
            "Geist Mono",
            "SF Mono",
            "Menlo",
            "Noto Sans Mono",
        ],
        // Graphite: the system's own neutral.
        ThemeId::Graphite => &[
            "Comic Mono",
            "ComicMono Nerd Font Mono",
            "SF Mono",
            "Geist Mono",
            "JetBrainsMono NF",
            "JetBrains Mono",
            "Menlo",
            "Noto Sans Mono",
        ],
        // Coldpress: flat, drafting-table — geometric and even.
        ThemeId::ColdpressGray => &[
            "Comic Mono",
            "ComicMono Nerd Font Mono",
            "FiraCode Nerd Font Mono",
            "Fira Code",
            "Google Sans Code",
            "SF Mono",
            "Menlo",
            "Noto Sans Mono",
        ],
        // Broadsheet / ledger: newsprint and accounting — a clean modern
        // humanist face, no old typewriter Courier.
        ThemeId::SalmonBroadsheet => &[
            "Comic Mono",
            "ComicMono Nerd Font Mono",
            "IBM Plex Mono",
            "MonoLisa",
            "Menlo",
            "Noto Sans Mono",
        ],
        ThemeId::IvoryLedger => &[
            "Comic Mono",
            "ComicMono Nerd Font Mono",
            "IBM Plex Mono",
            "SF Mono",
            "Menlo",
            "Noto Sans Mono",
        ],
        // CRT: a terminal face with squared-off shoulders — straight faces
        // only (no ligature/cursive `Cascadia Code`).
        ThemeId::CrtGreen | ThemeId::CrtAmber | ThemeId::CrtBlue | ThemeId::CrtViolet => &[
            "Comic Mono",
            "ComicMono Nerd Font Mono",
            "JetBrainsMono NF",
            "JetBrains Mono",
            "Google Sans Code",
            "Monaco",
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
    fn allowlist_has_no_typewriter_faces() {
        for banned in [
            "Courier",
            "Courier New",
            "PT Mono",
            "Andale Mono",
            "Consolas",
        ] {
            assert!(
                !FONT_ALLOWLIST.contains(&banned),
                "{banned} must not be auto-selectable"
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
