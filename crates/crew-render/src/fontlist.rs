//! The font picker's family list, with a deliberate inclusion policy.
//!
//! Candidates are faces flagged monospaced in their font tables OR whose name
//! reads as a coding face (variable fonts like JetBrains Mono often lack the
//! flag). But table flags also lie the other way: proportional Unicode
//! fallbacks (Arial Unicode MS), symbol/math faces (STIXNonUnicode) and
//! icon-only fonts (Symbols Nerd Font Mono) arrive flagged monospaced. So
//! every candidate is verified by measurement — it must actually render
//! fixed-pitch Latin text: `i`, `m` and `0` all present, all with the same
//! advance. What the picker lists is exactly what works as a terminal font.
use glyphon::cosmic_text::fontdb;
use glyphon::FontSystem;

/// Whether a family name reads as a coding/terminal face. Variable and
/// otherwise mis-flagged fonts (JetBrains Mono among them) often lack the
/// `monospaced` bit in their tables, so the picker would hide them; the name
/// heuristic keeps them as candidates.
pub(crate) fn sounds_monospace(name: &str) -> bool {
    let l = name.to_lowercase();
    [
        "mono", "consol", "courier", "menlo", "monaco", "code", "fixed", "term",
    ]
    .iter()
    .any(|h| l.contains(h))
}

/// Families crew refuses to offer at all — legacy typewriter/dated faces the
/// design never wants, whatever their font tables claim. Courier ships flagged
/// monospaced and passes the fixed-pitch check, so without this it would slip
/// into the picker and the `/font` rotation pool.
pub(crate) fn is_blocked(name: &str) -> bool {
    let l = name.to_lowercase();
    ["courier", "pt mono", "andale", "consolas"]
        .iter()
        .any(|b| l.contains(b))
}

/// Measured check: the face renders fixed-pitch Latin — `i`, `m` and `0` all
/// map to real glyphs and share one advance. Excludes proportional fallbacks
/// and symbol fonts whatever their tables claim.
fn fixed_pitch_latin(font_system: &mut FontSystem, id: fontdb::ID, weight: fontdb::Weight) -> bool {
    let Some(font) = font_system.get_font(id, weight) else {
        return false;
    };
    let swash = font.as_swash();
    let metrics = swash.glyph_metrics(&[]);
    let charmap = swash.charmap();
    let mut widths = ['i', 'm', '0'].into_iter().map(|c| {
        let gid = charmap.map(c);
        (gid != 0).then(|| metrics.advance_width(gid))
    });
    let Some(Some(first)) = widths.next() else {
        return false;
    };
    widths.all(|w| matches!(w, Some(x) if (x - first).abs() < 0.5))
}

/// Sorted, de-duplicated names of installed families that pass the policy:
/// candidate by flag or name, verified fixed-pitch by measurement.
pub(crate) fn monospace_families(font_system: &mut FontSystem) -> Vec<String> {
    let mut cand: Vec<(String, fontdb::ID, fontdb::Weight)> = font_system
        .db()
        .faces()
        .flat_map(|f| {
            let (mono, id, weight) = (f.monospaced, f.id, f.weight);
            f.families
                .iter()
                .filter(move |(name, _)| (mono || sounds_monospace(name)) && !is_blocked(name))
                .map(move |(name, _)| (name.clone(), id, weight))
        })
        .collect();
    cand.sort_by(|a, b| a.0.cmp(&b.0));
    cand.dedup_by(|a, b| a.0 == b.0);
    cand.retain(|(_, id, weight)| fixed_pitch_latin(font_system, *id, *weight));
    cand.into_iter().map(|(name, _, _)| name).collect()
}

#[cfg(test)]
#[path = "fontlist_tests.rs"]
mod tests;
