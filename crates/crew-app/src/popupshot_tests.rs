//! Offscreen shots of every composer pop-up standing on a crew pane's
//! composer — the model picker, the slash palette, the attach picker, Cmd+F,
//! Ctrl+R and the key prompt — each placed and sized by
//! `popupplace::scene`: on the composer's top edge, flush left, as wide as
//! its own rows. `#[ignore]`d: needs a GPU adapter.
//! `cargo test -p crew-app --bin crew popup_shot -- --ignored --nocapture`;
//! PNGs land in `$CREW_SHOT_DIR` (default `target/screenshots`).
use crate::chat::ChatPane;
use crate::goalshot_tests::dump;
use crate::layout::Rect;
use crate::popupplace::Popup;
use crew_render::PaneScene;

pub(crate) const W: u32 = 900;
const H: u32 = 520;

fn rect(w: u32) -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w: w as f32,
        h: H as f32,
    }
}

/// The frame: the pane, then the pop-up as `popupplace::scene` places it.
/// Returns the scenes and the overlay's rect.
pub(crate) fn frame(
    p: &ChatPane,
    w: u32,
    cw: f32,
    ch: f32,
    popup: Popup,
) -> (Vec<PaneScene>, Rect) {
    let r = rect(w);
    let cols = (r.w / cw).floor() as u16;
    let rows = (r.h / ch).floor() as u16;
    let over = crate::popupplace::scene(p, r, cw, ch, popup);
    let at = Rect {
        x: over.x,
        y: over.y,
        w: over.w,
        h: over.h,
    };
    let scenes = vec![
        PaneScene {
            cells: p.cells(cols, rows),
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
            focused: true,
            bordered: false,
            glass: false,
            scan: -1.0,
            overlay: false,
            paint: Vec::new(),
        },
        over,
    ];
    (scenes, at)
}

/// Whether any pixel in `x0..x1` × `y0..y1` differs from the page.
fn inked(px: &[u8], w: u32, x0: u32, x1: u32, y0: u32, y1: u32) -> bool {
    let page = &px[..4];
    (y0..y1.min(H)).any(|y| {
        (x0..x1.min(w)).any(|x| {
            let i = ((y * w + x) * 4) as usize;
            let d = |k: usize| (px[i + k] as i32 - page[k] as i32).abs();
            d(0) + d(1) + d(2) > 90
        })
    })
}

/// Shoot `popup` over `p` at width `w`, dump its rows, and check the two
/// placement rules on the pixels: the composer's `❯` still shows under the
/// card, and the pane to the RIGHT of the card is bare page (no band).
pub(crate) fn shot(name: &str, p: &ChatPane, w: u32, popup: impl Fn(u16) -> Popup) -> Option<()> {
    let mut at = rect(w);
    let (mut cw_px, mut ch_px) = (0.0, 0.0);
    let mut card_cols = 0u16;
    // Through the tube when the theme is one: the bloom is part of what a
    // frame looks like there, and a one-pixel rule is what a halo undoes.
    let draw = match crew_theme::current_id().is_crt() {
        true => crate::shotdraw_tests::draw_crt,
        false => crate::shotdraw_tests::draw,
    };
    let px = draw(w, H, 13.0, |cw, ch| {
        (cw_px, ch_px) = (cw, ch);
        let pop = popup((w as f32 / cw).floor() as u16);
        card_cols = pop.cols;
        eprintln!("--- {name} {}x{}", pop.cols, pop.rows);
        for l in dump(&pop.cells, pop.cols, pop.rows) {
            eprintln!("|{l}");
        }
        let (scenes, r) = frame(p, w, cw, ch, pop);
        at = r;
        scenes
    })?;
    crate::shotdraw_tests::write_png(name, &px, w, H);
    let bottom = at.y + at.h;
    // The composer's bordered card: top border row, then the `❯` row.
    let prompt_row = (bottom + ch_px) as u32;
    assert!(
        inked(
            &px,
            w,
            0,
            (ch_px * 3.0) as u32,
            prompt_row,
            prompt_row + ch_px as u32
        ),
        "{name}: the composer prompt is hidden under the pop-up"
    );
    // The frame is DRAWN: its vertical stroke stands on the last cell's far
    // edge, so the corner is looked for over the card's last column plus a
    // pixel or two, on the top border row.
    let (cw, ch) = (cw_px as u32, ch_px as u32);
    let (edge, top) = ((at.x + f32::from(card_cols) * cw_px) as u32, at.y as u32);
    assert!(
        inked(&px, w, edge - cw, edge + 2, top, top + ch),
        "{name}: the card's corner is drawn at its hugged edge"
    );
    // ...and NOT at the pane's edge: the old full-width band put a corner
    // there, on every pop-up, with nothing but page between.
    if edge + 2 * cw < w {
        assert!(
            !inked(&px, w, w - cw, w, top, top + ch),
            "{name}: a band of card runs to the pane's edge"
        );
    }
    Some(())
}
