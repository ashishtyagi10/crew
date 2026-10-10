//! Scene-building: converts PaneScene slice into quads + per-pane Buffers.
//! Shaped buffers are reused frame-to-frame when a pane's content signature is
//! unchanged (see [`crate::scenecache`]).
use glyphon::Buffer;

use unicode_width::UnicodeWidthChar;

use crate::cellgrid::{default_bg, CellView};
use crate::celltext::{build_pane_buffer, FontParams};
use crate::glass::GlassCard;
use crate::paint::Paint;
use crate::quads::Quad;
use crate::roundborder::Border;
use crate::scenecache::{pane_sig, PrevPass};

/// `(Buffer, origin_x, origin_y, pane_w, pane_h)` for one rendered pane.
pub(crate) type PaneBuffer = (Buffer, f32, f32, f32, f32);

/// One pane to be rendered: its cell data, pixel rect, and focus state.
/// Columns a cell's character occupies — the same one-or-two the layout
/// advances it by (see `celltext::cells_for`), as a multiplier for the marks
/// drawn under it.
fn cell_cols(c: char) -> f32 {
    match UnicodeWidthChar::width(c) {
        Some(2) => 2.0,
        _ => 1.0,
    }
}

#[derive(Clone)]
pub struct PaneScene {
    pub cells: Vec<CellView>,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub focused: bool,
    /// Whether to draw the rounded GPU border. Surfaces that draw their own
    /// cell-based border (e.g. the input bar's titled card) set this `false`.
    pub bordered: bool,
    /// Whether this scene is a pane's *card* — the scene covering the whole
    /// pane rect — and so gets the frosted sheet drawn beneath it.
    ///
    /// Deliberately NOT `bordered`: since panes started drawing their frames as
    /// cells (fieldset legends) every scene sets `bordered: false`, so gating
    /// the sheet on it meant no card was ever emitted: the Glass setting painted
    /// nothing in the app while the headless test — which builds its own
    /// `bordered: true` panes — kept passing.
    pub glass: bool,
    /// Position of the scan highlight sweeping this card, `0.0..=1.0`, or a
    /// negative value for none. Driven only while a pane is *working*: a busy
    /// pane already repaints (see the app's `poll`), so the sweep costs no
    /// extra frames, and an idle crew never draws it at all.
    pub scan: f32,
    /// How far this card has lifted off the page, `0.0..=2.0`: the focused
    /// pane rises to 1 and the one it left sinks back, on the focus clock;
    /// a floating card (pop-up, toast) rides at 2. Negative sinks it into the
    /// page as a well (the input bar, at -1).
    /// Deepens the glass sheet's shadow and brightens its rim; a card with no
    /// sheet ignores it.
    pub lift: f32,
    /// Where the focus glint has reached along the card's rim, `0.0..=1.0`
    /// (left to right), or negative for none: a soft specular that sweeps
    /// once along the edge as focus lands on a card, as light runs across
    /// glass when it tilts.
    pub glint: f32,
    /// Sub-cell vector rectangles drawn between this pane's cell backgrounds
    /// and its text — the layer charts are painted on. Cell units; see
    /// [`Paint`].
    pub paint: Vec<Paint>,
    /// A card's FRAME scene: its drawing fills the rect exactly, the sub-cell
    /// remainder spent inside the frame rather than left as gap beside it
    /// (see [`crate::stretch`]). Only scenes whose last column and row are
    /// the frame's own set this — content split off the far edge would read
    /// as a gap in the text.
    pub stretch: bool,
    /// Overlay popups (command palette, help) drawn on top of everything. Their
    /// backgrounds and text are rendered in a second pass *after* base panes, so
    /// nothing behind them can bleed through — they are fully opaque.
    pub overlay: bool,
}

/// An empty, unfocused, frameless scene with no scan and no lift — what a
/// literal spells out field by field, so one can name only what it sets.
impl Default for PaneScene {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            focused: false,
            bordered: false,
            glass: false,
            scan: -1.0,
            lift: 0.0,
            glint: -1.0,
            paint: Vec::new(),
            stretch: false,
            overlay: false,
        }
    }
}

const BORDER_RADIUS: f32 = 10.0;

/// Where a frame rule's stroke centre falls inside a cell `extent` px across —
/// the same whole-pixel placement `boxglyph` draws `─` and `│` at, so the
/// glass sheet's edge lands under the line rather than beside it. `cell_h`
/// sizes the stroke, as it does for the glyphs.
fn stroke_centre(extent: f32, cell_h: f32) -> f32 {
    let t = crate::boxglyph::light_thickness(cell_h.round() as u32);
    let (lo, _) = crate::boxglyph::centre(extent.round() as u32, t);
    lo as f32 + t as f32 / 2.0
}

/// [`stroke_centre`] on both axes of a `cell_w`×`cell_h` cell.
pub(crate) fn stroke_inset(cell_w: f32, cell_h: f32) -> (f32, f32) {
    (stroke_centre(cell_w, cell_h), stroke_centre(cell_h, cell_h))
}

/// One built pass: quads, buffers (with this frame's signatures), borders and
/// the frosted-glass cards drawn beneath them.
type ScenePass = (
    Vec<Quad>,
    Vec<PaneBuffer>,
    Vec<u64>,
    Vec<Border>,
    Vec<GlassCard>,
);

/// [`build_scene`] for both passes at once: `(base, overlay)`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_both(
    panes: &[PaneScene],
    cell_w: f32,
    cell_h: f32,
    font_system: &mut glyphon::FontSystem,
    params: &FontParams,
    srgb: bool,
    glass: crew_theme::GlassStyle,
    window_opacity: f32,
    prev_base: PrevPass,
    prev_overlay: PrevPass,
) -> (ScenePass, ScenePass) {
    let base = build_scene(
        panes,
        cell_w,
        cell_h,
        font_system,
        params,
        false,
        srgb,
        glass,
        window_opacity,
        prev_base,
    );
    let overlay = build_scene(
        panes,
        cell_w,
        cell_h,
        font_system,
        params,
        true,
        srgb,
        glass,
        window_opacity,
        prev_overlay,
    );
    (base, overlay)
}

/// Build all quads (cell backgrounds) and one Buffer per pane, plus rounded borders.
/// Returns `(quads, pane_buffers, sigs, borders)`. Only panes whose `overlay`
/// flag equals `want_overlay` are built, so the caller can render base panes
/// and overlay popups as two separate passes. `srgb` names the target format
/// (colours convert once at this boundary — see [`crate::color`]); `prev` is
/// last frame's `(sigs, buffers)`, reused slot-by-slot on a signature match so
/// unchanged panes skip re-shaping entirely.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_scene(
    panes: &[PaneScene],
    cell_w: f32,
    cell_h: f32,
    font_system: &mut glyphon::FontSystem,
    params: &FontParams,
    want_overlay: bool,
    srgb: bool,
    // Already scaled by the user's glass level; derived from the active theme
    // at the frame layer (`CellGrid::set_scene`) so this stays theme-agnostic.
    glass_style: crew_theme::GlassStyle,
    // Below 1 a liquid slab's body lets the desktop through
    // (`LiquidStyle::body`); at 1 it is solid, as it always was.
    window_opacity: f32,
    prev: PrevPass,
) -> ScenePass {
    let mut quads: Vec<Quad> = Vec::new();
    let mut buffers: Vec<PaneBuffer> = Vec::new();
    let mut sigs: Vec<u64> = Vec::new();
    let mut borders: Vec<Border> = Vec::new();
    let mut cards: Vec<GlassCard> = Vec::new();
    let (prev_sigs, prev_bufs) = prev;
    let mut prev_bufs: Vec<Option<PaneBuffer>> = prev_bufs.into_iter().map(Some).collect();

    for pane in panes {
        if pane.overlay != want_overlay {
            continue;
        }
        let cols = ((pane.w / cell_w).floor() as usize).max(1);
        let rows = ((pane.h / cell_h).floor() as usize).max(1);
        let split = crate::stretch::split(pane, cell_w, cell_h);
        // The corners that round at card scale, and the scene as DRAWN: their
        // glyphs blanked for the arcs to stand in. The sheet still reads the
        // scene as written — a blanked corner is not a legend's gap.
        let corners = crate::corners::find(&pane.cells, cell_w, cell_h);
        let blank;
        let drawn = if corners.is_empty() {
            pane
        } else {
            blank = crate::corners::blanked(pane, &corners);
            &blank
        };
        let round = crate::corners::radius(cell_w, cell_h);

        // Overlay popups get a solid black backdrop spanning the whole pane,
        // drawn before their cell quads. The overlay pass runs after all base
        // text, so this fully occludes anything behind — a 100%-opaque box. A
        // pure-black per-cell bg wouldn't suffice: cells skip the bg quad when
        // their colour is the default, and base text would still show through.
        if pane.overlay {
            let bg = crew_theme::theme().page_bg;
            let color = crate::color::target_rgba(bg, 1.0, srgb);
            // Rounded with the frame inside it, concentric: the stroke's
            // inset plus the corners' radius.
            let (ix, _) = stroke_inset(cell_w, cell_h);
            quads.push(Quad {
                radii: [round + ix; 4],
                ..Quad::rect(pane.x, pane.y, pane.w, pane.h, color)
            });
        }

        // The frosted sheet this pane sits on. Only card scenes get one — a
        // pane contributes several scenes (content, frame) and one sheet per
        // pane is the point — and overlay popups are deliberately opaque so
        // nothing behind them bleeds through (see `PaneScene::overlay`).
        //
        // An overlay gets one too — but only its shadow: a floating card is
        // opaque, it is its own sheet, and the overlay pass draws the glass
        // OVER its backgrounds (see `CellGrid::draw`) so the shadow can fall
        // across the page margin a pop-up keeps beside its frame. Drawn over
        // them, a rim would strike through the legend's own backing.
        if pane.glass && glass_style.visible() {
            // The sheet spans the *drawn* card, not the raw rect: fieldset
            // frames are cell-quantized (`floor(px/cell)` per axis), so a
            // full-rect sheet overhangs the border by up to a cell — a bright
            // edge outside the frame that reads as a second, misaligned box.
            //
            // And not the drawn card's OUTER edge either, but the frame's
            // STROKE: the `─`/`│` rules sit mid-cell, so a sheet out to the
            // cell edge left half a cell of glass — and its shadow — outside
            // the line, which is the "second, misaligned box" all over again.
            // Inset to the stroke's centre and rounded like the `╭` arc, the
            // sheet's edge runs under the frame and the frame hides it.
            let (ix, iy) = (stroke_centre(cell_w, cell_h), stroke_centre(cell_h, cell_h));
            let sheet = if pane.overlay { 0.0 } else { 1.0 };
            // A stretched frame reaches its rect's far edges, and so does
            // its sheet.
            //
            // Edge to edge from the first column's stroke to the LAST
            // column's — each `ix` into its own cell — not to `ix` short of
            // the frame's far edge: a cell is rarely `2 * ix` wide, and the
            // difference drew the rim a pixel beside the right-hand rule.
            let (fw, fh, fcols, frows) = match split {
                Some(s) => (
                    f32::from(s.lc) * cell_w + s.sx + 2.0 * ix,
                    f32::from(s.lr) * cell_h + s.sy + 2.0 * iy,
                    usize::from(s.lc) + 1,
                    usize::from(s.lr) + 1,
                ),
                None => (
                    (cols - 1) as f32 * cell_w + 2.0 * ix,
                    (rows - 1) as f32 * cell_h + 2.0 * iy,
                    cols,
                    rows,
                ),
            };
            let mut notch = crate::notch::notch(&pane.cells, fcols, frows, cell_w, cell_h, ix, iy);
            if let Some(s) = split {
                crate::notch::shift(&mut notch, f32::from(s.lc) * cell_w - ix, s.sx);
            }
            cards.push(GlassCard {
                x: pane.x + ix,
                y: pane.y + iy,
                w: (fw - 2.0 * ix).max(0.0),
                h: (fh - 2.0 * iy).max(0.0),
                // The corners' own radius: the frame and its glass bend as one.
                radius: round,
                alpha_top: glass_style.alpha_top * sheet,
                alpha_bottom: glass_style.alpha_bottom * sheet,
                noise: glass_style.noise,
                tint: crate::color::target_rgba(glass_style.tint, 1.0, srgb),
                highlight: crate::color::target_rgba(glass_style.highlight, 1.0, srgb),
                highlight_alpha: glass_style.highlight_alpha * sheet,
                shadow_alpha: glass_style.shadow_alpha,
                scan: pane.scan,
                edge_glow: glass_style.edge_glow,
                gloss: glass_style.gloss * sheet,
                glow: glass_style.glow,
                etch: glass_style.etch * sheet,
                lift: pane.lift,
                glint: pane.glint,
                notch,
                // How much of the desktop the body lets through. A pop-up's
                // sheet is only its shadow (`sheet` 0): it has no body.
                see: match crew_theme::theme().liquid {
                    Some(l) if window_opacity < 1.0 => 1.0 - l.body,
                    _ => 0.0,
                },
                liquid: crew_theme::theme().liquid.is_some() && sheet > 0.0,
            });
        }

        // Rounded-corner border for this pane (unless it draws its own).
        if pane.bordered {
            let t = crew_theme::theme();
            let rgb = if pane.focused {
                t.border_focused
            } else {
                t.border_normal
            };
            let color = crate::color::target_rgba(rgb, 1.0, srgb);
            borders.push(Border {
                x: pane.x,
                y: pane.y,
                w: pane.w,
                h: pane.h,
                radius: BORDER_RADIUS,
                thickness: t.border_thickness,
                color,
                color_h: None,
                clip: None,
            });
        }

        // The cells themselves — once, or once per slice of a stretched
        // frame, with the rules carried across the stretch.
        let first = buffers.len();
        let parts = split.map(|s| crate::stretch::parts(drawn, &s, cell_w, cell_h));
        for part in parts.as_deref().unwrap_or(std::slice::from_ref(drawn)) {
            emit_cells(
                part,
                cell_w,
                cell_h,
                font_system,
                params,
                // A pop-up is solid (its backdrop above): no glass in it.
                (
                    srgb,
                    crate::crtchain::premultiplies(window_opacity) && !pane.overlay,
                ),
                &prev_sigs,
                &mut prev_bufs,
                &mut Out {
                    quads: &mut quads,
                    buffers: &mut buffers,
                    sigs: &mut sigs,
                },
            );
        }
        if let Some(s) = split {
            for (x, y, w, h, fg) in crate::stretch::bridges(drawn, &s, cell_w, cell_h) {
                quads.push(Quad::rect(
                    x,
                    y,
                    w,
                    h,
                    crate::color::target_rgba(fg, 1.0, srgb),
                ));
            }
        }
        // The arcs, placed on the very glyphs their tails run into.
        let at = crate::corners::Drawn {
            pane: drawn,
            split: split.as_ref(),
            buffers: &buffers[first..],
            cell_w,
            cell_h,
        };
        borders.extend(crate::corners::arcs(&at, &corners, srgb));
    }

    (quads, buffers, sigs, borders, cards)
}

/// Where [`emit_cells`] writes: the pass's quads, and its text buffers with
/// their signatures, slot for slot.
struct Out<'a> {
    quads: &'a mut Vec<Quad>,
    buffers: &'a mut Vec<PaneBuffer>,
    sigs: &'a mut Vec<u64>,
}

/// How far an inline mark's capsule reaches past its cells on each side (px).
const MARK_PAD: f32 = 2.0;

/// One scene's cells into quads and a text buffer: backgrounds as runs, the
/// rules each cell wears, the vector paint, then the shaped text — last
/// frame's buffer when the slot's signature matches.
#[allow(clippy::too_many_arguments)]
fn emit_cells(
    pane: &PaneScene,
    cell_w: f32,
    cell_h: f32,
    font_system: &mut glyphon::FontSystem,
    params: &FontParams,
    // The target's sRGB-ness, and whether the window is sheer glass.
    (srgb, sheer): (bool, bool),
    prev_sigs: &[u64],
    prev_bufs: &mut [Option<PaneBuffer>],
    out: &mut Out<'_>,
) {
    let cols = ((pane.w / cell_w).floor() as usize).max(1);
    let rows = ((pane.h / cell_h).floor() as usize).max(1);
    let quads = &mut *out.quads;
    // Cell backgrounds, as runs: one quad per horizontal run of a colour,
    // its corners rounded wherever they sit on bare page (`bgruns`).
    let (gcols, grows) = pane.cells.iter().fold((cols, rows), |(c, r), cell| {
        let end = usize::from(cell.col) + cell_cols(cell.c) as usize;
        (c.max(end), r.max(usize::from(cell.row) + 1))
    });
    let rad = crate::bgruns::radius(cell_w, cell_h);
    // On a sheer glass window a tint of the page is a frost, not a slab.
    let fill = |bg| crate::color::target_rgba(bg, crate::cellveil::alpha(bg, sheer), srgb);
    for run in crate::bgruns::runs(&pane.cells, gcols, grows, default_bg()) {
        let radii = run.round.map(|r| if r { rad } else { 0.0 });
        // A mark keeps a pixel of its row's edge either side (`bgruns::Run`)
        // and breathes MARK_PAD px past its glyphs: at the cell's own width a
        // one-cell chip was a disc narrower than its letter, `w` and `/`
        // clipped at the curve.
        let (inset, pad) = if run.mark {
            (1.0, MARK_PAD)
        } else {
            (0.0, 0.0)
        };
        quads.push(Quad {
            x: pane.x + f32::from(run.col) * cell_w - pad,
            y: pane.y + f32::from(run.row) * cell_h + inset,
            w: f32::from(run.cols) * cell_w + 2.0 * pad,
            h: cell_h - 2.0 * inset,
            color: fill(run.bg),
            radii,
        });
    }

    // Background quads for cells with non-default bg colour, then the
    // rules the cell wears (underline family, strikethrough). The rules go
    // after the cell's own background so they are never buried by it, and
    // before the text pass so a descender crosses them the way it does in
    // print.
    for cell in &pane.cells {
        let x = pane.x + f32::from(cell.col) * cell_w;
        let y = pane.y + f32::from(cell.row) * cell_h;
        // Everything a cell wears has to cover every column the cell
        // occupies. A full-width character owns TWO, and the second
        // carries no `CellView` of its own — the terminal drops
        // alacritty's spacer and every widget places one cell per
        // character — so a mark measured in one cell left a gap on the
        // other: a selection over Japanese was a row of stripes, an
        // underline broke under every wide glyph, and a TUI's painted
        // status bar came out perforated.
        let w = cell_cols(cell.c) * cell_w;
        if !cell.deco.is_blank() {
            let rgb = crate::deco::color(&cell.deco, cell.fg);
            let color = crate::color::target_rgba(rgb, 1.0, srgb);
            for (x, y, w, h) in crate::deco::rects(&cell.deco, x, y, w, cell_h) {
                quads.push(Quad::rect(x, y, w, h, color));
            }
        }
        if cell.cursor.is_rule() {
            let color = crate::color::target_rgba(cell.cursor.color, 1.0, srgb);
            for (x, y, w, h) in crate::deco::cursor_rects(&cell.cursor, x, y, w, cell_h) {
                quads.push(Quad::rect(x, y, w, h, color));
            }
        }
    }

    // The pane's vector paint: cell-unit rectangles scaled by this frame's
    // cell size. After the cell backgrounds so a chart is not buried by the
    // page it sits on, and before the text pass so labels read on top of it.
    for p in pane.paint.iter().filter(|p| p.visible()) {
        quads.push(Quad::rect(
            pane.x + p.x * cell_w,
            pane.y + p.y * cell_h,
            p.w * cell_w,
            p.h * cell_h,
            crate::color::target_rgba(p.color, p.alpha, srgb),
        ));
    }

    // One text Buffer per pane — last frame's, when the signature matches
    // (position is not part of the signature; a moved pane reuses too).
    let sig = pane_sig(pane, cols, rows, params);
    let slot = out.buffers.len();
    let buf = match prev_sigs.get(slot) == Some(&sig) {
        true => prev_bufs.get_mut(slot).and_then(Option::take),
        false => None,
    };
    let buf = buf.map(|b| b.0).unwrap_or_else(|| {
        build_pane_buffer(font_system, &pane.cells, cols, rows, pane.w, pane.h, params)
    });
    out.sigs.push(sig);
    out.buffers.push((buf, pane.x, pane.y, pane.w, pane.h));
}

#[cfg(test)]
#[path = "scene_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "scene_sig_tests.rs"]
mod sig_tests;
