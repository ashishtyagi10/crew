//! Programmer ligatures (`=>`, `!=`, `->`, `===`) come from the font's own
//! `liga`/`calt`/`clig` lookups, which the shaper runs by default over every
//! run of same-style cells — so a face that has them (Lilex, crew's built-in,
//! does) already draws them, each glyph spanning exactly its cells because
//! `set_monospace_width` snaps every advance to the cell. What crew owes the
//! user is the OFF switch, done here.
//!
//! The caret does not split a ligature (Kitty and WezTerm do). The shaper only
//! breaks a run on family, stretch, style or weight, and every way to force a
//! break was worse than a joined glyph: a different `Stretch` picks a narrower
//! face in families that have one (Iosevka, Monaspace), and a blinking caret
//! would join and split the glyph on every blink.
use glyphon::cosmic_text::{Attrs, FeatureTag, FontFeatures};

/// Features that turn every ligature lookup off for the run they ride on.
fn off() -> FontFeatures {
    let mut f = FontFeatures::new();
    for tag in [
        FeatureTag::STANDARD_LIGATURES,
        FeatureTag::CONTEXTUAL_LIGATURES,
        FeatureTag::CONTEXTUAL_ALTERNATES,
    ] {
        f.disable(tag);
    }
    f
}

/// `a`, shaped as the ligature setting asks. Off goes on EVERY span, the
/// blank ones too: the shaper takes a run's features from its first span.
pub(crate) fn shape(a: Attrs<'_>, enabled: bool) -> Attrs<'_> {
    match enabled {
        true => a,
        false => a.font_features(off()),
    }
}
