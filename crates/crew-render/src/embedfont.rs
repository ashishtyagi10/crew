//! The typeface crew ships with itself, so the app never depends on what the
//! machine happens to have installed.
//!
//! ## Why this exists
//!
//! cosmic-text hardcodes `Family::Monospace` to the family name
//! `"Noto Sans Mono"` (`cosmic-text/src/font/system.rs`). On a machine without
//! it the query simply misses, and shaping falls through to the platform's
//! *common* fallback list — which on Windows is `["Segoe UI", …]`, a
//! **proportional** face. (macOS is no better: its list leads with `.SF NS`.
//! macOS only escaped the bug because `Menlo` sits in every theme's
//! preference list and is always installed there, so resolution never
//! actually reached the fallback.) `celltext` then applies
//! `set_monospace_width`, which rounds every advance to the nearest cell:
//! a proportional face's narrow glyphs (`i`, `l`, `.`, `|`) round to *zero*,
//! so glyphs collide and columns drift. That is what made a fresh Windows
//! install look mangled — the banner and the nav cards worst of all, since
//! box-drawn frames expose column drift instantly.
//!
//! Theme font preferences could not save it either: they name faces stock on
//! macOS and Linux (`SF Mono`, `Menlo`, `Noto Sans Mono`), and `resolve_family`
//! correctly declines to apply a family that isn't installed.
//!
//! So crew embeds [`crew_theme::EMBEDDED_FAMILY`] and registers it as *the*
//! monospace family. Installed fonts are now an enhancement — a theme
//! preference or `/font` pick still wins — rather than a requirement.
//!
//! ## Why static faces, not the variable font
//!
//! `fontdb` does not read `fvar`: a variable font registers as one face at its
//! default weight, so every `/weight` step would render identically. The four
//! upright statics plus italics cover the configurable 300–900 range through
//! CSS weight matching. See `assets/fonts/README.md` for provenance.
use glyphon::cosmic_text::fontdb;
use glyphon::FontSystem;
use std::sync::Arc;

/// The embedded faces, as `(name, bytes)` — name is for test failure messages
/// only; the family comes from each face's own tables.
pub(crate) const EMBEDDED: [(&str, &[u8]); 8] = [
    (
        "Regular",
        include_bytes!("../../../assets/fonts/Lilex-Regular.otf"),
    ),
    (
        "Italic",
        include_bytes!("../../../assets/fonts/Lilex-Italic.otf"),
    ),
    (
        "Medium",
        include_bytes!("../../../assets/fonts/Lilex-Medium.otf"),
    ),
    (
        "MediumItalic",
        include_bytes!("../../../assets/fonts/Lilex-MediumItalic.otf"),
    ),
    (
        "SemiBold",
        include_bytes!("../../../assets/fonts/Lilex-SemiBold.otf"),
    ),
    (
        "SemiBoldItalic",
        include_bytes!("../../../assets/fonts/Lilex-SemiBoldItalic.otf"),
    ),
    (
        "Bold",
        include_bytes!("../../../assets/fonts/Lilex-Bold.otf"),
    ),
    (
        "BoldItalic",
        include_bytes!("../../../assets/fonts/Lilex-BoldItalic.otf"),
    ),
];

/// The embedded faces as fontdb sources.
pub(crate) fn sources() -> impl Iterator<Item = fontdb::Source> {
    EMBEDDED
        .iter()
        .map(|(_, bytes)| fontdb::Source::Binary(Arc::new(*bytes)))
}

/// A `FontSystem` with the embedded faces loaded and registered as the
/// monospace default.
///
/// **Every** `FontSystem` in crew must come from here. One built with
/// `FontSystem::new()` silently reverts to the Noto-Sans-Mono-or-Segoe-UI
/// behaviour above, and a test using one would be measuring a different app
/// than the one that ships.
pub fn font_system() -> FontSystem {
    let mut fs = FontSystem::new_with_fonts(sources());
    // Overrides cosmic-text's own `set_monospace_family("Noto Sans Mono")`,
    // which `new_with_fonts` applies *after* loading — so this must come
    // after construction, not before.
    fs.db_mut()
        .set_monospace_family(crew_theme::EMBEDDED_FAMILY);
    fs
}

#[cfg(test)]
#[path = "embedfont_tests.rs"]
mod tests;
