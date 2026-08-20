//! Does this family actually render one glyph per cell — *as crew shapes it*?
//!
//! ## Why measuring the font is not enough
//!
//! `fontlist` already screens candidates by measuring `i`/`m`/`0` in the face's
//! own tables. That check passed on a Windows box that then rendered the whole
//! app in a proportional face: the picker offered a family by name, crew
//! applied it, and at *shape* time the name resolved to something else
//! entirely — so what got measured and what got drawn were different fonts.
//!
//! A name can fail to resolve for more reasons than are worth enumerating
//! (localized family names, a weight the family does not carry, a subset or
//! broken face installed under a familiar name). The layout is where the truth
//! is, so that is where crew now checks.
//!
//! ## What goes wrong when it is wrong
//!
//! `celltext` asks cosmic-text to round every advance to the nearest multiple
//! of one cell:
//!
//! ```text
//! x_advance = round(x_advance / cell_w) * cell_w
//! ```
//!
//! For a fixed-pitch face at crew's 0.6 ratio, every glyph is ~1.0 cells and
//! lands on exactly one. For a *proportional* face, `m` and `w` are ~1.4 cells
//! and round to one, while `i`, `l`, `.` and **space** are ~0.43 cells and
//! round to **zero** — they stack on top of their neighbour. That is the
//! reported symptom exactly: `terminals.` drawn as `term inals`, `commands` as
//! `com m ands`, and every space in the log line gone.
use glyphon::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Weight};

/// Characters that separate a fixed-pitch face from a proportional one: the
/// narrowest glyphs crew draws (these are the ones that round to zero) and the
/// widest (these are the ones that steal a second cell), plus the space, whose
/// collapse is what makes the damage obvious.
const PROBE: &str = "il.'| mMW0 ";

/// Whether every glyph of [`PROBE`] shaped in `family` occupies exactly one
/// cell of `cell_w` at `font_size`/`weight`.
///
/// `false` means crew must not use this family: it would draw a grid with
/// glyphs piled on top of each other. Shapes through the same buffer settings
/// as `celltext::build_pane_buffer`, so a pass here is a pass in the app.
pub(crate) fn snaps_to_cells(
    font_system: &mut FontSystem,
    family: &str,
    font_size: f32,
    cell_w: f32,
    weight: u16,
) -> bool {
    if family.is_empty() || cell_w <= 0.0 || font_size <= 0.0 {
        return false;
    }
    let mut buf = Buffer::new(font_system, Metrics::new(font_size, font_size * 1.25));
    // The same quantum celltext uses — see its `set_monospace_width` note.
    buf.set_monospace_width(font_system, Some(cell_w * font_size));
    buf.set_text(
        font_system,
        PROBE,
        &Attrs::new()
            .family(Family::Name(family))
            .weight(Weight(weight)),
        Shaping::Advanced,
        None,
    );
    buf.shape_until_scroll(font_system, false);
    let Some(run) = buf.layout_runs().next() else {
        return false;
    };
    if run.glyphs.len() != PROBE.chars().count() {
        return false; // shaped away characters — not a terminal face
    }
    run.glyphs.iter().all(|g| (g.w - cell_w).abs() < 0.01)
}

#[cfg(test)]
#[path = "fontverify_tests.rs"]
mod tests;
