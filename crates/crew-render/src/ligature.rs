//! Programmer ligatures (`=>`, `!=`, `->`, `===`) come from the font's own
//! `liga`/`calt`/`clig` lookups, which the shaper runs by default over every
//! run of same-style cells — so a face that has them (Lilex, crew's built-in,
//! does) already draws them, each glyph spanning exactly its cells because
//! `set_monospace_width` snaps every advance to the cell. What crew owes the
//! user is the OFF switch and the cursor's rule, both done here.
use glyphon::cosmic_text::{Attrs, FeatureTag, FontFeatures, Stretch};

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

/// `a`, shaped as the ligature setting and the cursor ask.
///
/// Setting off: the features go on every run. Cursor cell: it must not join
/// its neighbours (Kitty and WezTerm do the same), so the caret sits on the
/// component character it is really at instead of covering half a joined
/// glyph. The shaper only breaks a run where family, stretch, style or weight
/// differ — NOT colour or features, which it takes from the run's first span
/// — so the cursor cell takes a different `Stretch`. Stretch is not in the
/// glyph cache key and the embedded face has one width, so it changes
/// nothing about how the cell is drawn.
pub(crate) fn shape(mut a: Attrs<'_>, enabled: bool, cursor: bool) -> Attrs<'_> {
    if !enabled {
        a = a.font_features(off());
    }
    if cursor {
        a = a.stretch(Stretch::SemiCondensed);
    }
    a
}
