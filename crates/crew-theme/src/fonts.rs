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
#[path = "fonts_tests.rs"]
mod tests;
