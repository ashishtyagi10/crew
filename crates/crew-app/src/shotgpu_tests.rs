//! Off-screen GPU render harness shared by the chart and sidebar shots, so a
//! widget can be *looked at* rather than only asserted on.
//!
//! Callers pick the canvas size: a chart wants a wide card, the left nav wants
//! a tall narrow column the shape the app actually docks. Everything else —
//! the paper background, the grain, the card the content is laid into — is the
//! same path `build_frame` draws a real frame with. The GPU plumbing under
//! this lives in `shotdraw_tests`, which a widget drawing its own card shoots
//! through directly.
use crew_render::{CellView, Paint, PaneScene};

/// Render one card filling a `w`×`h` shot: `content` returns the interior's
/// cells and paint at the `(cols, rows, aspect)` it is given, exactly as a
/// sidebar section does. Returns RGBA pixels, or `None` where there is no GPU.
pub fn render_at(
    w: u32,
    h: u32,
    font_px: f32,
    legend: &str,
    content: impl FnOnce(u16, u16, f32) -> (Vec<CellView>, Vec<Paint>),
) -> Option<Vec<u8>> {
    crate::shotdraw_tests::draw(w, h, font_px, |cw, ch| {
        let rect = crate::layout::Rect {
            x: 12.0,
            y: 12.0,
            w: w as f32 - 24.0,
            h: h as f32 - 24.0,
        };
        let mut scenes: Vec<PaneScene> = Vec::new();
        crate::panelcard::push_card_art(
            &mut scenes,
            rect,
            cw,
            ch,
            legend,
            crew_theme::theme().legend_off,
            |cols, rows| content(cols, rows, ch / cw),
        );
        scenes
    })
}

/// Render and write `<name>.png` under `$CREW_SHOT_DIR`, returning the pixels.
pub fn shot_at(
    name: &str,
    w: u32,
    h: u32,
    font_px: f32,
    legend: &str,
    content: impl FnOnce(u16, u16, f32) -> (Vec<CellView>, Vec<Paint>),
) -> Option<Vec<u8>> {
    let px = render_at(w, h, font_px, legend, content)?;
    crate::shotdraw_tests::write_png(name, &px, w, h);
    Some(px)
}

/// Count pixels that differ from the page background by more than the grain —
/// "did this widget put ink on the page at all".
pub fn ink(px: &[u8]) -> usize {
    let bg = crew_theme::theme().page_bg;
    px.chunks_exact(4)
        .filter(|p| {
            (p[0] as i32 - bg.0 as i32).abs()
                + (p[1] as i32 - bg.1 as i32).abs()
                + (p[2] as i32 - bg.2 as i32).abs()
                > 40
        })
        .count()
}
