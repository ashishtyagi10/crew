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
    /// Sub-cell vector rectangles drawn between this pane's cell backgrounds
    /// and its text — the layer charts are painted on. Cell units; see
    /// [`Paint`].
    pub paint: Vec<Paint>,
    /// Overlay popups (command palette, help) drawn on top of everything. Their
    /// backgrounds and text are rendered in a second pass *after* base panes, so
    /// nothing behind them can bleed through — they are fully opaque.
    pub overlay: bool,
}

const BORDER_RADIUS: f32 = 10.0;

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

        // Overlay popups get a solid black backdrop spanning the whole pane,
        // drawn before their cell quads. The overlay pass runs after all base
        // text, so this fully occludes anything behind — a 100%-opaque box. A
        // pure-black per-cell bg wouldn't suffice: cells skip the bg quad when
        // their colour is the default, and base text would still show through.
        if pane.overlay {
            let bg = crew_theme::theme().page_bg;
            quads.push(Quad {
                x: pane.x,
                y: pane.y,
                w: pane.w,
                h: pane.h,
                color: crate::color::target_rgba(bg, 1.0, srgb),
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
            if cell.bg != default_bg() {
                quads.push(Quad {
                    x,
                    y,
                    w,
                    h: cell_h,
                    color: crate::color::target_rgba(cell.bg, 1.0, srgb),
                });
            }
            if !cell.deco.is_blank() {
                let rgb = crate::deco::color(&cell.deco, cell.fg);
                let color = crate::color::target_rgba(rgb, 1.0, srgb);
                for (x, y, w, h) in crate::deco::rects(&cell.deco, x, y, w, cell_h) {
                    quads.push(Quad { x, y, w, h, color });
                }
            }
            if cell.cursor.is_rule() {
                let color = crate::color::target_rgba(cell.cursor.color, 1.0, srgb);
                for (x, y, w, h) in crate::deco::cursor_rects(&cell.cursor, x, y, w, cell_h) {
                    quads.push(Quad { x, y, w, h, color });
                }
            }
        }

        // The pane's vector paint: cell-unit rectangles scaled by this frame's
        // cell size. After the cell backgrounds so a chart is not buried by the
        // page it sits on, and before the text pass so labels read on top of it.
        for p in pane.paint.iter().filter(|p| p.visible()) {
            quads.push(Quad {
                x: pane.x + p.x * cell_w,
                y: pane.y + p.y * cell_h,
                w: p.w * cell_w,
                h: p.h * cell_h,
                color: crate::color::target_rgba(p.color, p.alpha, srgb),
            });
        }

        // The frosted sheet this pane sits on. Only card scenes get one — a
        // pane contributes several scenes (content, frame) and one sheet per
        // pane is the point — and overlay popups are deliberately opaque so
        // nothing behind them bleeds through (see `PaneScene::overlay`).
        if pane.glass && !pane.overlay && glass_style.visible() {
            // The sheet spans the *drawn* card, not the raw rect: fieldset
            // frames are cell-quantized (`floor(px/cell)` per axis), so a
            // full-rect sheet overhangs the border by up to a cell — a bright
            // edge outside the frame that reads as a second, misaligned box.
            cards.push(GlassCard {
                x: pane.x,
                y: pane.y,
                w: cols as f32 * cell_w,
                h: rows as f32 * cell_h,
                radius: BORDER_RADIUS,
                alpha_top: glass_style.alpha_top,
                alpha_bottom: glass_style.alpha_bottom,
                noise: glass_style.noise,
                tint: crate::color::target_rgba(glass_style.tint, 1.0, srgb),
                highlight: crate::color::target_rgba(glass_style.highlight, 1.0, srgb),
                highlight_alpha: glass_style.highlight_alpha,
                shadow_alpha: glass_style.shadow_alpha,
                scan: pane.scan,
                edge_glow: glass_style.edge_glow,
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
            });
        }

        // One text Buffer per pane — last frame's, when the signature matches
        // (position is not part of the signature; a moved pane reuses too).
        let sig = pane_sig(pane, cols, rows, params);
        let slot = buffers.len();
        let buf = match prev_sigs.get(slot) == Some(&sig) {
            true => prev_bufs.get_mut(slot).and_then(Option::take),
            false => None,
        };
        let buf = buf.map(|b| b.0).unwrap_or_else(|| {
            build_pane_buffer(font_system, &pane.cells, cols, rows, pane.w, pane.h, params)
        });
        sigs.push(sig);
        buffers.push((buf, pane.x, pane.y, pane.w, pane.h));
    }

    (quads, buffers, sigs, borders, cards)
}

/// The rect of the card the eye is on — `[x, y, w, h]` in physical px — or
/// `None` when nothing focused asks for one. What the frame layer solidifies
/// when the window is sheer (see [`crate::solidcard`]).
///
/// Two rules, and both are about matching what is actually DRAWN:
///
/// * **Quantized to whole cells**, for the same reason the glass sheet is: a
///   card's frame is drawn as cells, so it ends at `floor(px / cell)`, and the
///   raw rect overhangs it by up to a cell — a band of solid page outside the
///   frame, which reads as a second, misaligned box.
/// * **The largest focused card wins.** Typing in the input bar leaves both
///   the bar and the pane it acts on focused (the spotlight deliberately keeps
///   that pane lit), and the surface being READ is the big one.
pub(crate) fn focused_card_rect(panes: &[PaneScene], cell_w: f32, cell_h: f32) -> Option<[f32; 4]> {
    if cell_w <= 0.0 || cell_h <= 0.0 {
        return None;
    }
    panes
        .iter()
        .filter(|p| p.focused && p.glass && !p.overlay)
        .map(|p| {
            let cols = (p.w / cell_w).floor().max(1.0);
            let rows = (p.h / cell_h).floor().max(1.0);
            [p.x, p.y, cols * cell_w, rows * cell_h]
        })
        .max_by(|a, b| (a[2] * a[3]).total_cmp(&(b[2] * b[3])))
}

#[cfg(test)]
#[path = "scene_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "scene_sig_tests.rs"]
mod sig_tests;
