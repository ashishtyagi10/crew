//! A run of joining script shaped on its own: at its natural advance, in
//! the family, weight, colour and coverage its cells would have had, and
//! smaller only when it has to fit a width. Cached by what it was shaped
//! from, so a pane that changed elsewhere, or scrolled a line, finds its
//! words already shaped.
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use glyphon::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, Style, Weight, Wrap};

use crate::celltext::{family_from, weight_in_family, FontParams};
use crate::scriptrun::RunSpec;

thread_local! {
    static SHAPED: RefCell<HashMap<u64, (Arc<Buffer>, f32)>> = RefCell::new(HashMap::new());
}

/// Runs kept at most; past it the cache starts over.
const KEEP: usize = 2048;

fn key(s: &RunSpec, p: &FontParams, room: f32) -> u64 {
    let mut h = std::hash::DefaultHasher::new();
    (&s.text, s.fg, s.weight, s.italic, s.light, s.gamma).hash(&mut h);
    (
        room.to_bits(),
        p.font_size.to_bits(),
        p.line_height.to_bits(),
    )
        .hash(&mut h);
    (&p.family, p.smooth).hash(&mut h);
    h.finish()
}

/// `s` shaped, and its baseline below its line's top: at its natural
/// advance when that fits `room` (pass `f32::INFINITY` for no limit), else
/// again at the size that does.
pub(crate) fn shaped(
    font_system: &mut FontSystem,
    s: &RunSpec,
    p: &FontParams,
    room: f32,
) -> (Arc<Buffer>, f32) {
    let k = key(s, p, room);
    if let Some(hit) = SHAPED.with(|m| m.borrow().get(&k).cloned()) {
        return hit;
    }
    let mut size = p.font_size;
    let mut buf = shape_at(font_system, s, p, size);
    // Advances scale with the size, near enough that a second try lands.
    for _ in 0..2 {
        let w = width(&buf);
        if w <= room {
            break;
        }
        size *= room / w * 0.995;
        buf = shape_at(font_system, s, p, size);
    }
    let base = buf
        .layout_runs()
        .next()
        .map_or(0.0, |r| r.line_y - r.line_top);
    let out = (Arc::new(buf), base);
    SHAPED.with(|m| {
        let mut m = m.borrow_mut();
        if m.len() >= KEEP {
            m.clear();
        }
        m.insert(k, out.clone());
    });
    out
}

/// How wide `buf`'s text is drawn.
pub(crate) fn width(buf: &Buffer) -> f32 {
    buf.layout_runs().map(|r| r.line_w).fold(0.0, f32::max)
}

/// `s` at `size`, its weight taken from the family that draws it at the body
/// weight. Fallback picks a family per weight: on macOS the body's 500 is
/// Kohinoor Devanagari and 700 Devanagari Sangam MN, so a bold headline
/// came out in another typeface and hardly heavier than the text under it.
fn shape_at(font_system: &mut FontSystem, s: &RunSpec, p: &FontParams, size: f32) -> Buffer {
    let body = shape_in(font_system, s, p, size, family_from(&p.family), p.weight);
    if s.weight == p.weight {
        return body;
    }
    let primary = p.family.as_deref().filter(|f| !f.is_empty());
    let primary = primary.unwrap_or(crew_theme::EMBEDDED_FAMILY);
    let found = body
        .layout_runs()
        .flat_map(|r| r.glyphs.iter())
        .find_map(|g| {
            let face = font_system.db().face(g.font_id)?;
            let name = face.families.first()?.0.clone();
            (name != primary).then_some(name)
        });
    match found {
        Some(name) => {
            let family = Some(name);
            let weight = weight_in_family(font_system, &family, s.weight).0;
            shape_in(font_system, s, p, size, family_from(&family), weight)
        }
        None => shape_in(font_system, s, p, size, family_from(&p.family), s.weight),
    }
}

fn shape_in(
    font_system: &mut FontSystem,
    s: &RunSpec,
    p: &FontParams,
    size: f32,
    family: Family<'_>,
    weight: u16,
) -> Buffer {
    let mut buf = Buffer::new(font_system, Metrics::new(size, p.line_height));
    buf.set_wrap(font_system, Wrap::None);
    buf.set_size(font_system, None, Some(p.line_height));
    let mut attrs = Attrs::new()
        .family(family)
        .weight(Weight(weight))
        .color(Color::rgb(s.fg.0, s.fg.1, s.fg.2))
        .cache_key_flags(crate::smoothing::text_flags(p.smooth, s.gamma, s.light));
    if s.italic {
        attrs = attrs.style(Style::Italic);
    }
    buf.set_text(font_system, &s.text, &attrs, Shaping::Advanced, None);
    buf
}
