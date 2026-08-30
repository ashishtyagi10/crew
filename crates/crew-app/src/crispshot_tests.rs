//! The crispness probe: how many pixels does one of crew's *rules* take to go
//! from page to ink and back?
//!
//! Every frame, divider, meter and bar in crew is a box-drawing character —
//! `─ │ ╭ █ ▍` — and every one of them travels the same pipeline a letter
//! does: swash rasterizes the font's outline, [`crew_render`]'s stem
//! darkening spills coverage into the neighbouring pixels, and the coverage
//! curve lifts what is left. All three are calibrated for *letterforms*. A
//! rule is not a letterform: it is a rectangle, and a rectangle drawn on the
//! pixel grid should have hard edges.
//!
//! This probe renders a card and reads the luminance across its top rule and
//! down its left one, so "soft" is a number rather than a squint.
//!
//! `#[ignore]`d (needs a GPU adapter):
//! `cargo test -p crew-app --bin crew crisp -- --ignored --test-threads=1`
use crew_render::PaneScene;

const PAD: f32 = 14.0;

/// Perceptual luminance of the pixel at `(x, y)` in an RGBA buffer.
fn luma_at(px: &[u8], w: u32, x: u32, y: u32) -> f32 {
    let i = ((y * w + x) * 4) as usize;
    0.2126 * px[i] as f32 + 0.7152 * px[i + 1] as f32 + 0.0722 * px[i + 2] as f32
}

/// One rule's cross-section, as coverage in `0.0..=1.0`: how far each pixel
/// travelled from the page toward full ink.
///
/// Averaged ALONG the rule (`runs` samples per offset) before anything is
/// read off it. The paper grain is per-pixel noise of the same order as a
/// rule's own fringe — a single-column slice reads 0.2 of "coverage" on bare
/// page — and a rule is constant along its length, so averaging leaves the
/// profile and cancels the grain.
fn profile(px: &[u8], w: u32, at: impl Fn(u32, u32) -> (u32, u32), n: u32, runs: u32) -> Vec<f32> {
    let page: f32 = (0..runs)
        .map(|k| luma_at(px, w, 2 + k % 4, 2 + k / 4))
        .sum::<f32>()
        / runs as f32;
    let raw: Vec<f32> = (0..n)
        .map(|i| {
            let s: f32 = (0..runs)
                .map(|k| {
                    let (x, y) = at(i, k);
                    luma_at(px, w, x, y)
                })
                .sum();
            s / runs as f32 - page
        })
        .collect();
    let peak = raw.iter().fold(0.0_f32, |a, b| a.max(b.abs()));
    raw.iter().map(|v| (v / peak.max(1.0)).abs()).collect()
}

/// Pixels whose coverage sits strictly between `lo` and `hi` — the fringe. A
/// crisp rule has at most one such pixel per edge; a dilated one has three.
fn fringe(p: &[f32], lo: f32, hi: f32) -> usize {
    p.iter().filter(|v| **v > lo && **v < hi).count()
}

fn card_px(w: u32, h: u32) -> Option<Vec<u8>> {
    crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| {
        let cols = ((w as f32 - 2.0 * PAD) / cw).floor() as u16;
        let rows = ((h as f32 - 2.0 * PAD) / ch).floor() as u16;
        vec![PaneScene {
            cells: crate::modernring::gradient_card(
                cols,
                rows,
                "CRISP",
                crew_theme::theme().border_normal,
                crew_theme::theme().legend_off,
                crew_theme::theme().page_bg,
            ),
            x: PAD,
            y: PAD,
            w: cols as f32 * cw,
            h: rows as f32 * ch,
            focused: false,
            bordered: false,
            glass: false,
            scan: -1.0,
            overlay: false,
            paint: Vec::new(),
        }]
    })
}

#[test]
#[ignore = "needs a GPU adapter"]
fn card_rules_are_hard_edged() {
    let _g = crate::app::theme_test_guard();
    let (w, h) = (600u32, 300u32);
    let Some(px) = card_px(w, h) else {
        eprintln!("no GPU adapter — skipped");
        return;
    };
    crate::shotdraw_tests::write_png("crisp_card", &px, w, h);
    let top = profile(&px, w, |i, k| (100 + k, i), 40, 300);
    println!("top rule, averaged over 300 columns, rows 0..40:");
    for (y, v) in top.iter().enumerate() {
        println!("  y={y:2} {v:.3} {}", "#".repeat((v * 40.0) as usize));
    }
    let left = profile(&px, w, |i, k| (i, 60 + k), 40, 180);
    println!("left rule, averaged over 180 rows, cols 0..40:");
    for (x, v) in left.iter().enumerate() {
        println!("  x={x:2} {v:.3} {}", "#".repeat((v * 40.0) as usize));
    }
    let (ft, fl) = (fringe(&top, 0.05, 0.95), fringe(&left, 0.05, 0.95));
    println!("fringe pixels (0.05..0.95): top={ft} left={fl}");
    // Before `crew_render`'s box glyphs were drawn instead of read from the
    // font, these read 3 and 4: the rule spanned four rows of pixels with one
    // row at full ink (0.20 / 0.78 / 1.00 / 0.25) and the same across. A
    // rectangle on the pixel grid has no fringe at all.
    assert_eq!((ft, fl), (0, 0), "the card frame is not a hard-edged rule");
    assert_eq!(
        (
            top.iter().filter(|v| **v > 0.95).count(),
            left.iter().filter(|v| **v > 0.95).count()
        ),
        (1, 1),
        "a light rule is one pixel of full ink, no more and no less"
    );
}

/// A column of one character, so a stem can be averaged down its own length.
fn text_px(w: u32, h: u32, c: char, rows: u16) -> Option<Vec<u8>> {
    crate::shotdraw_tests::draw(w, h, 13.0, move |cw, ch| {
        let t = crew_theme::theme();
        let cells: Vec<crew_render::CellView> = (0..rows)
            .map(|row| crew_render::CellView {
                col: 0,
                row,
                c,
                fg: t.ink,
                bg: t.page_bg,
                ..Default::default()
            })
            .collect();
        vec![PaneScene {
            cells,
            x: PAD,
            y: PAD,
            w: cw * 4.0,
            h: ch * rows as f32,
            focused: false,
            bordered: false,
            glass: false,
            scan: -1.0,
            overlay: false,
            paint: Vec::new(),
        }]
    })
}

/// The letterform half of the same question: how many pixels does a vertical
/// STEM take to go from page to ink?
///
/// A stem is not a rule — it is a typeface's own shape at a typeface's own
/// position, so some fringe is correct and the number here is a watch rather
/// than a target. What it is watching for is the stem darkening quietly
/// widening: the dilation in `crew_render::smoothmask` spills coverage a
/// fraction of a pixel sideways, and a stem that reads across five pixels
/// with none at full ink is a stem that has been smeared, not darkened.
#[test]
#[ignore = "needs a GPU adapter"]
fn a_stem_reaches_full_ink() {
    let _g = crate::app::theme_test_guard();
    let (w, h) = (200u32, 300u32);
    let Some(px) = text_px(w, h, 'l', 16) else {
        eprintln!("no GPU adapter — skipped");
        return;
    };
    crate::shotdraw_tests::write_png("crisp_stem", &px, w, h);
    let stem = profile(&px, w, |i, k| (i, PAD as u32 + 4 + k), 24, 200);
    println!("stem of 'l', averaged over 200 rows:");
    for (x, v) in stem.iter().enumerate() {
        println!("  x={x:2} {v:.3} {}", "#".repeat((v * 40.0) as usize));
    }
    let full = stem.iter().filter(|v| **v > 0.95).count();
    let fr = fringe(&stem, 0.08, 0.95);
    println!("stem: {full} px at full ink, {fr} fringe px");
    assert!(full >= 1, "the stem never reaches full ink: {stem:?}");
}
