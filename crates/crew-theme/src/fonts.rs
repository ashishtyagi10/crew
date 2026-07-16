//! Each theme's preferred monospace families, most-wanted first.
//!
//! A theme cannot name ONE font: fonts are per-machine. A miss makes fontdb
//! substitute a proportional face, and cell rounding then mangles every glyph
//! — so a theme states an ordered preference and the app takes the first
//! family that is actually installed (`crew-app`'s `theme_font`), changing
//! nothing when none of them are. Pure data: resolving needs the renderer's
//! installed-family list, which lives in `crew-app`, not here.
//!
//! Every list ends in faces that ship with the OS (`Menlo`/`SF Mono` on macOS,
//! `Noto Sans Mono`/`DejaVu Sans Mono` on Linux, `Consolas` on Windows) so a
//! bare machine still resolves something rather than silently opting out.
use crate::ThemeId;

/// The families this theme would like, best first. Empty = no opinion.
pub fn font_prefs(id: ThemeId) -> &'static [&'static str] {
    match id {
        // Paper: a book face — humanist, generous counters.
        ThemeId::PaperDark | ThemeId::PaperLight => &[
            "IBM Plex Mono",
            "Fragment Mono",
            "Menlo",
            "Noto Sans Mono",
            "DejaVu Sans Mono",
            "Consolas",
        ],
        // Sepia: warm and typewritten.
        ThemeId::SepiaDark | ThemeId::SepiaLight => &[
            "PT Mono",
            "IBM Plex Mono",
            "Courier New",
            "Menlo",
            "Noto Sans Mono",
            "DejaVu Sans Mono",
        ],
        // Midnight ink: high-contrast, tight.
        ThemeId::MidnightInk => &[
            "JetBrainsMono NF",
            "JetBrains Mono",
            "Geist Mono",
            "Menlo",
            "Noto Sans Mono",
            "DejaVu Sans Mono",
        ],
        // Graphite: the system's own neutral.
        ThemeId::Graphite => &[
            "SF Mono",
            "Geist Mono",
            "Menlo",
            "Noto Sans Mono",
            "DejaVu Sans Mono",
            "Consolas",
        ],
        // Coldpress: flat, drafting-table.
        ThemeId::ColdpressGray => &[
            "Fragment Mono",
            "Google Sans Code",
            "Menlo",
            "Noto Sans Mono",
            "DejaVu Sans Mono",
        ],
        // Broadsheet / ledger: newsprint and accounting.
        ThemeId::SalmonBroadsheet => &[
            "PT Mono",
            "Courier New",
            "Courier",
            "Menlo",
            "Noto Sans Mono",
            "DejaVu Sans Mono",
        ],
        ThemeId::IvoryLedger => &[
            "Courier",
            "Courier New",
            "PT Mono",
            "Menlo",
            "Noto Sans Mono",
            "DejaVu Sans Mono",
        ],
        // CRT: a terminal face with squared-off shoulders.
        ThemeId::CrtGreen | ThemeId::CrtAmber | ThemeId::CrtBlue | ThemeId::CrtViolet => &[
            "Andale Mono",
            "Monaco",
            "Menlo",
            "Noto Sans Mono",
            "DejaVu Sans Mono",
            "Consolas",
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
            "Consolas",
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
