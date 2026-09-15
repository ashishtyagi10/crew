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
/// `font_pool`). ONE canonical name per typeface: the intersection is taken by
/// [`typeface_key`], so `Comic Mono` here answers for `ComicMono Nerd Font
/// Mono` on the machine. It used to list every spelling by hand, which meant a
/// face was reachable only in the spellings somebody had thought to add.
///
/// Deliberately excludes typewriter/legacy faces (Courier, Courier New, PT
/// Mono, Andale, Consolas, and pre-Retina Monaco — SF Mono is the modern
/// macOS face), Stelo (its lowercase `l` renders as a broken bar — user bug
/// report 2026-07-24) and Intel One Mono with its `IntoneMono` Nerd Font
/// builds (taste — asked out 2026-09-06): a rotation must never land on one. The *manual*
/// `/font` picker is unaffected — it still offers every installed coding
/// face; this only governs what crew picks on its own. Menlo and the other
/// OS-stock faces stay ONLY as mid-list options — never a lead, and no longer
/// the safety net either ([`EMBEDDED_FAMILY`] is).
pub const FONT_ALLOWLIST: &[&str] = &[
    "JetBrains Mono",
    "Menlo",
    "Berkeley Mono",
    "Cascadia Code",
    "Cascadia Mono",
    "Comic Mono",
    "Commit Mono",
    "Fira Code",
    "Geist Mono",
    "Google Sans Code",
    "IBM Plex Mono",
    "Lilex",
    "Martian Mono",
    "MonoLisa",
    "Noto Sans Mono",
    "Operator Mono",
    "Roboto Mono",
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
        // Paper: a book face — humanist, generous counters. Every list names
        // typefaces, not spellings: `resolve_family` matches by
        // `typeface_key`, and the pool holds the best installed spelling of
        // each (the icon-bearing build where there is one).
        ThemeId::PaperDark | ThemeId::PaperLight => &[
            "MonoLisa",
            "IBM Plex Mono",
            "Comic Mono",
            "SF Mono",
            "Menlo",
            "Lilex",
        ],
        // Sepia: warm and typewritten — friendly rounded shapes suit it, so
        // this is where the Comic Mono lead lives on.
        ThemeId::SepiaDark | ThemeId::SepiaLight => &[
            "Comic Mono",
            "IBM Plex Mono",
            "MonoLisa",
            "SF Mono",
            "Menlo",
            "Lilex",
        ],
        // Modern (aurora/nebula): the Gemini look wants a contemporary
        // geometric coding face, and JetBrains Mono is that face — it also
        // happens to be one the user asked to see more of (`FAVORITES`),
        // which is the tie-breaker between it and Google's own. Google Sans
        // Code and Geist stay behind it.
        ThemeId::Nebula | ThemeId::Blossom => &[
            "JetBrains Mono",
            "SF Mono",
            "Google Sans Code",
            "Geist Mono",
            "Menlo",
            "Lilex",
        ],
        // Harbor and Fern are the modern page COOLED, and they used to share
        // Nebula's list outright. They lead with IBM Plex Mono instead: an
        // engineered, level face for the cooler page, and a lead of their own
        // is a font change when the rotation moves between the two halves of
        // the modern family.
        ThemeId::Harbor | ThemeId::Fern => &[
            "IBM Plex Mono",
            "SF Mono",
            "Google Sans Code",
            "Geist Mono",
            "Menlo",
            "Lilex",
        ],
        // CRT: a terminal face with squared-off shoulders — straight modern
        // faces (the old `Monaco` lead was a pre-Retina relic; Lilex is the
        // contemporary take on that IBM-terminal DNA).
        // The light twins share their dark parents' faces — a palette flip
        // must not also change the typeface under the user.
        ThemeId::CrtGreen | ThemeId::CrtAmber | ThemeId::CrtBlue | ThemeId::CrtViolet => &[
            // One entry, and it is the face crew embeds. That used to be
            // spelled `["Lilex Nerd Font", "Lilex"]` so an installed icon
            // build would beat the built-in copy — resolution matches by
            // typeface now and the pool already holds the better spelling, so
            // the second name was the same face written twice.
            "Lilex",
        ],
    }
}

#[cfg(test)]
#[path = "fonts_tests.rs"]
mod tests;
