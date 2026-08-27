use super::*;
use crate::cellgrid::{default_bg, CellView};
use crate::celltext::FontParams;
use crew_theme::{GlassLevel, GlassStyle};
use glyphon::FontSystem;

/// A fully transparent style — what paper themes derive; builds no cards.
pub(super) fn no_glass() -> GlassStyle {
    GlassStyle {
        tint: (0, 0, 0),
        alpha_top: 0.0,
        alpha_bottom: 0.0,
        highlight: (0, 0, 0),
        highlight_alpha: 0.0,
        shadow_alpha: 0.0,
        noise: 0.0,
        edge_glow: 0.0,
    }
}

/// A visible style, passed explicitly so these tests never depend on which
/// theme happens to be globally active (paper derives an invisible one).
fn test_glass() -> GlassStyle {
    GlassStyle {
        tint: (120, 200, 160),
        alpha_top: 0.25,
        alpha_bottom: 0.10,
        highlight: (200, 255, 220),
        highlight_alpha: 0.30,
        shadow_alpha: 0.0,
        noise: 0.0,
        edge_glow: 0.35,
    }
}

fn cell(col: u16, row: u16, c: char, bg: (u8, u8, u8)) -> CellView {
    CellView {
        col,
        row,
        c,
        fg: (200, 200, 200),
        bg,
        bold: false,
        italic: false,
        ..Default::default()
    }
}

fn params() -> FontParams {
    FontParams {
        font_size: 14.0,
        line_height: 17.5,
        cell_w: 14.0 * 0.6,
        family: None,
        weight: 400,
        smooth: 0,
    }
}

fn pane(cells: Vec<CellView>, bordered: bool, overlay: bool) -> PaneScene {
    PaneScene {
        cells,
        x: 0.0,
        y: 0.0,
        w: 80.0,
        h: 40.0,
        focused: false,
        bordered,
        glass: false,
        scan: -1.0,
        overlay,
    }
}

/// A pane's *card* scene — the one spanning the whole rect, which is what asks
/// for the frosted sheet. Separate from `bordered`: real cards draw their frame
/// as cells and set `bordered: false`.
fn card(cells: Vec<CellView>, bordered: bool, overlay: bool) -> PaneScene {
    PaneScene {
        glass: true,
        scan: -1.0,
        ..pane(cells, bordered, overlay)
    }
}

/// `build_scene` with the arguments these tests never vary. `glass` stays
/// explicit — several tests below are about exactly that argument.
fn build(
    panes: &[PaneScene],
    fs: &mut FontSystem,
    want_overlay: bool,
    glass: GlassStyle,
) -> ScenePass {
    build_scene(
        panes,
        8.0,
        16.0,
        fs,
        &params(),
        want_overlay,
        false,
        glass,
        (vec![], vec![]),
    )
}

#[test]
fn bg_quads_only_for_non_default_cells() {
    let mut fs = crate::embedfont::font_system();
    let panes = vec![pane(
        vec![cell(0, 0, 'a', default_bg()), cell(1, 0, 'b', (10, 20, 30))],
        false,
        false,
    )];
    let (quads, buffers, _sigs, borders, _cards) = build(&panes, &mut fs, false, no_glass());
    assert_eq!(quads.len(), 1, "only the non-default-bg cell gets a quad");
    assert_eq!(buffers.len(), 1);
    assert!(borders.is_empty());
    assert_eq!(quads[0].x, 8.0); // positioned at col 1
    assert_eq!(quads[0].color[3], 1.0); // opaque
}

#[test]
fn bordered_pane_emits_a_border() {
    let mut fs = crate::embedfont::font_system();
    let (_q, _b, _s, borders, _c) = build(&[pane(vec![], true, false)], &mut fs, false, no_glass());
    assert_eq!(borders.len(), 1);
}

#[test]
fn want_overlay_partitions_panes() {
    let mut fs = crate::embedfont::font_system();
    let panes = vec![
        pane(vec![cell(0, 0, 'x', (1, 2, 3))], true, false),
        pane(vec![cell(0, 0, 'y', (4, 5, 6))], false, true),
    ];
    // Base pass: only the non-overlay pane (bordered → one border).
    let (q, b, _s, bd, _c) = build(&panes, &mut fs, false, no_glass());
    assert_eq!((q.len(), b.len(), bd.len()), (1, 1, 1));
    // Overlay pass: only the overlay pane (bordered:false → no border). Two
    // quads: the full-rect black backdrop plus the one non-default-bg cell.
    let (q2, b2, _s2, bd2, _c2) = build(&panes, &mut fs, true, no_glass());
    assert_eq!((q2.len(), b2.len(), bd2.len()), (2, 1, 0));
}

#[test]
fn overlay_pane_gets_an_opaque_page_bg_backdrop() {
    let mut fs = crate::embedfont::font_system();
    // An overlay pane with only default-bg cells still gets a backdrop.
    let panes = vec![pane(vec![cell(0, 0, 'y', default_bg())], false, true)];
    let (quads, _b, _s, _bd, _c) = build(&panes, &mut fs, true, no_glass());
    assert_eq!(quads.len(), 1, "the backdrop quad, no per-cell quad");
    let q = &quads[0];
    assert_eq!((q.x, q.y, q.w, q.h), (0.0, 0.0, 80.0, 40.0)); // spans the pane
    let t = crew_theme::theme();
    let (r, g, b) = t.page_bg;
    assert_eq!(
        q.color,
        [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
    );
}

#[test]
fn focused_border_is_brighter_than_unfocused() {
    let mut fs = crate::embedfont::font_system();
    let mut p = pane(vec![], true, false);
    p.focused = true;
    let (_q, _b, _s, focused, _c) = build(&[p], &mut fs, false, no_glass());
    let (_q2, _b2, _s2, normal, _c2) =
        build(&[pane(vec![], true, false)], &mut fs, false, no_glass());
    let t = crew_theme::theme();
    let f = |c: (u8, u8, u8)| {
        [
            c.0 as f32 / 255.0,
            c.1 as f32 / 255.0,
            c.2 as f32 / 255.0,
            1.0,
        ]
    };
    assert_eq!(focused[0].color, f(t.border_focused));
    assert_eq!(normal[0].color, f(t.border_normal));
}

// --- glass ------------------------------------------------------------------

/// The sheet spans the cell-quantized *drawn* frame, not the raw rect: with
/// 8×16 cells an 80×40 pane draws floor(40/16) = 2 rows, so the card is
/// 80×32. A full-rect sheet overhung the frame by up to a cell — the
/// "misaligned input bar" phantom edge.
#[test]
fn glass_card_matches_the_drawn_frame_and_border_radius() {
    let mut fs = crate::embedfont::font_system();
    let (_q, _b, _s, borders, cards) =
        build(&[card(vec![], true, false)], &mut fs, false, test_glass());
    assert_eq!(cards.len(), 1);
    let c = &cards[0];
    assert_eq!((c.x, c.y, c.w, c.h), (0.0, 0.0, 80.0, 32.0));
    // The sheet must share the border's radius exactly, or the frost and the
    // stroke drift apart at the corners.
    assert_eq!(c.radius, borders[0].radius);
}

#[test]
fn glass_off_builds_no_cards() {
    let mut fs = crate::embedfont::font_system();
    let (_q, _b, _s, _bd, cards) = build(&[card(vec![], true, false)], &mut fs, false, no_glass());
    assert!(cards.is_empty(), "Off must cost nothing to draw");
}

/// A pane contributes several scenes (cell-inset content, full-rect card) and
/// only the card asks for a sheet — otherwise every pane would be frosted twice,
/// the inner sheet darkening a band one cell inside the frame.
#[test]
fn non_card_scenes_get_no_glass() {
    let mut fs = crate::embedfont::font_system();
    let (_q, _b, _s, _bd, cards) =
        build(&[pane(vec![], true, false)], &mut fs, false, test_glass());
    assert!(cards.is_empty());
}

/// Overlay popups are deliberately opaque so nothing behind them bleeds
/// through — glass under one would undo that.
#[test]
fn overlay_panes_get_no_glass() {
    let mut fs = crate::embedfont::font_system();
    let (_q, _b, _s, _bd, cards) = build(&[card(vec![], true, true)], &mut fs, true, test_glass());
    assert!(cards.is_empty());
}

/// The user's `/glass` level scales the style before it reaches this layer;
/// the card must carry whatever alphas it was handed.
#[test]
fn level_scales_the_cards_it_builds() {
    let mut fs = crate::embedfont::font_system();
    let mut built = |style| {
        let (_q, _b, _s, _bd, cards) = build(&[card(vec![], true, false)], &mut fs, false, style);
        cards.into_iter().next().expect("expected a glass card")
    };
    let low = built(test_glass().scaled(GlassLevel::Low)).alpha_top;
    let high = built(test_glass().scaled(GlassLevel::High)).alpha_top;
    assert!(low < high, "Low {low} should be fainter than High {high}");
}

/// The scan rides the card instance, so a working pane's sheet can sweep
/// without any second draw call.
#[test]
fn the_scan_position_reaches_the_card() {
    let mut fs = crate::embedfont::font_system();
    let mut p = card(vec![], true, false);
    p.scan = 0.4;
    let (_q, _b, _s, _bd, cards) = build(&[p], &mut fs, false, test_glass());
    assert_eq!(cards.len(), 1);
    assert!((cards[0].scan - 0.4).abs() < 1e-6);
}

/// A resting card carries no scan — the shader skips it on a negative value,
/// which is what keeps an idle crew's sheet completely still.
#[test]
fn a_resting_card_has_no_scan() {
    let mut fs = crate::embedfont::font_system();
    let (_q, _b, _s, _bd, cards) =
        build(&[card(vec![], true, false)], &mut fs, false, test_glass());
    assert!(cards[0].scan < 0.0);
}

/// A decorated cell puts its rule on the canvas as quads. Cell backgrounds are
/// the only other quads in this pane, so the count difference is the rule.
#[test]
fn a_decorated_cell_adds_quads_and_a_plain_one_does_not() {
    use crew_theme::deco::{Deco, DecoLine};
    let mut fs = crate::embedfont::font_system();
    let plain = cell(0, 0, 'x', default_bg());
    let underlined = CellView {
        deco: Deco::underline(DecoLine::Single),
        ..cell(0, 0, 'x', default_bg())
    };
    let curly = CellView {
        deco: Deco::underline(DecoLine::Curly),
        ..cell(0, 0, 'x', default_bg())
    };
    let mut count = |c: CellView| {
        let panes = vec![pane(vec![c], false, false)];
        build(&panes, &mut fs, false, no_glass()).0.len()
    };
    assert_eq!(count(plain), 0, "an undecorated cell draws no quad at all");
    assert_eq!(count(underlined), 1, "one rule is one quad");
    assert!(
        count(curly) > 4,
        "the squiggle is sampled across the cell, not drawn as one bar"
    );
}

/// SGR 58's colour reaches the GPU: the rule's quad is red where the glyph is
/// grey. `target_rgba` is the same conversion the cell backgrounds take.
#[test]
fn the_rule_is_drawn_in_the_underline_colour_not_the_text_colour() {
    use crew_theme::deco::{Deco, DecoLine};
    let mut fs = crate::embedfont::font_system();
    let c = CellView {
        deco: Deco {
            color: Some((255, 0, 0)),
            ..Deco::underline(DecoLine::Single)
        },
        ..cell(0, 0, 'x', default_bg())
    };
    let panes = vec![pane(vec![c], false, false)];
    let quads = build(&panes, &mut fs, false, no_glass()).0;
    assert_eq!(quads.len(), 1);
    assert_eq!(
        quads[0].color,
        crate::color::target_rgba((255, 0, 0), 1.0, false)
    );
    assert_ne!(
        quads[0].color,
        crate::color::target_rgba((200, 200, 200), 1.0, false)
    );
}
