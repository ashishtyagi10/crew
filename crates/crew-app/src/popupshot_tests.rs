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

/// The frame the app draws: the pane as a card whose content is
/// `popupband::art` (the rows the pop-up stands on cleared beside it, as
/// `pane.rs` draws a chat pane), then the pop-up OPEN ON THE PANE
/// (`ChatPane::popup`, as `render.rs` asks for it) placed by
/// `popupplace::scene`. Returns the scenes, the overlay's rect and the card.
///
/// It drew `p.cells` and a free-standing pop-up until survey UI #16: every
/// shot showed transcript beside the card that the live app clears.
pub(crate) fn frame(p: &ChatPane, w: u32, cw: f32, ch: f32, now: u64) -> (Vec<PaneScene>, Rect) {
    let r = rect(w);
    let pop = p
        .popup((r.w / cw).floor() as u16)
        .expect("the shot opens a pop-up on the pane");
    let mut scenes = Vec::new();
    let ink = crew_theme::theme().legend_off;
    crate::panelcard::push_card_art(&mut scenes, r, cw, ch, "1 crew", ink, |cols, rows| {
        let g = crew_term::GridSize { cols, rows };
        crate::popupband::art(p, true, g, ch / cw)
    });
    let over = crate::popupplace::scene(p, r, cw, ch, pop, now);
    let at = Rect {
        x: over.x,
        y: over.y,
        w: over.w,
        h: over.h,
    };
    scenes.push(over);
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

/// Shoot the pop-up open on `p` at width `w`, dump its rows, and check the
/// placement rules on the pixels: the composer's `❯` still shows under the
/// card, and the rows RIGHT of the card are bare (no band, no transcript).
pub(crate) fn shot(name: &str, p: &ChatPane, w: u32) -> Option<()> {
    shot_at(name, p, w, 0)
}

/// [`shot`] with the frame's clock at `now` — a rising card is shot
/// mid-rise, so the placement checks below allow for its drop.
pub(crate) fn shot_at(name: &str, p: &ChatPane, w: u32, now: u64) -> Option<()> {
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
        let pop = p
            .popup((w as f32 / cw).floor() as u16)
            .expect("the shot opens a pop-up on the pane");
        card_cols = pop.cols;
        eprintln!("--- {name} {}x{}", pop.cols, pop.rows);
        for l in dump(&pop.cells, pop.cols, pop.rows) {
            eprintln!("|{l}");
        }
        let (scenes, r) = frame(p, w, cw, ch, now);
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
    // ...and the rows it stands on are bare from past its edge to inside
    // the pane's own border: no band of card (the old full-width band put a
    // corner at the pane's edge) and no transcript tail (`popupband` clears
    // it; `p.cells` drew it) — its bottom row too, where the card stands.
    // Measured from where the card SETTLES: a rising card is drawn `drop`
    // low, the band it clears is not.
    let settled = (bottom - p.popup_rise.drop_rows(now) * ch_px) as u32;
    if edge + 3 * cw < w {
        let (x0, x1) = (edge + 2 * cw, w - 2 * cw);
        assert!(
            !inked(&px, w, x0, x1, top + ch / 2, settled),
            "{name}: something is drawn beside the pop-up on the rows it stands on"
        );
        // The row under the card is the composer's top border, drawn all
        // the way to the composer's right corner. The card stood on that
        // row (`above_composer` measured the frame, not the content) and
        // `popupband` cleared it, so the border stopped at the card. (A
        // tile the card all but fills leaves no run of border to look at.)
        assert!(
            x0 + cw >= w - 3 * cw || inked(&px, w, x0, w - 3 * cw, settled, settled + ch),
            "{name}: the composer's top border is gone beside the pop-up"
        );
    }
    Some(())
}
