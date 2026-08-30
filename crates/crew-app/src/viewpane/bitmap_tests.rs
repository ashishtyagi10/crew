//! What a picture drawn out of rectangles has to get right: it must land
//! inside the pane, keep the proportions of the file, and not cost a quad per
//! sample when the samples are all the same.
use super::*;

/// A bitmap of `w`×`h` filled by `f`, without going through a decoder.
fn bm(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4] + Copy) -> Bitmap {
    Bitmap {
        w,
        h,
        px: (0..h).flat_map(|y| (0..w).map(move |x| f(x, y))).collect(),
        src: (w, h),
    }
}

/// The frame's `cell_h / cell_w`: a cell is about twice as tall as it is wide.
const ASPECT: f32 = 2.1;

fn bounds(ps: &[Paint]) -> (f32, f32, f32, f32) {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in ps {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x + p.w);
        y1 = y1.max(p.y + p.h);
    }
    (x0, y0, x1, y1)
}

#[test]
fn a_picture_stays_inside_the_pane_it_was_given() {
    let img = bm(40, 30, |x, y| [x as u8, y as u8, 128, 255]);
    for (cols, rows) in [(20u16, 10u16), (80, 40), (7, 3), (200, 60)] {
        let ps = paint(&img, cols, rows, ASPECT);
        let (x0, y0, x1, y1) = bounds(&ps);
        assert!(x0 >= -0.01 && y0 >= -0.01, "{cols}x{rows}: starts off-pane");
        assert!(
            x1 <= f32::from(cols) + 0.01 && y1 <= f32::from(rows) + 0.01,
            "{cols}x{rows}: runs past the pane ({x1}, {y1})"
        );
    }
}

/// The whole reason `aspect` is a parameter: cells are twice as tall as they
/// are wide, so a square picture drawn on a square *grid* comes out twice as
/// tall as it is.
#[test]
fn a_square_picture_is_drawn_square_on_a_grid_of_tall_cells() {
    let ps = paint(&bm(32, 32, |_, _| [200, 100, 50, 255]), 60, 40, ASPECT);
    let (x0, y0, x1, y1) = bounds(&ps);
    let (w_sq, h_sq) = (x1 - x0, (y1 - y0) * ASPECT);
    assert!(
        (w_sq - h_sq).abs() < 0.2,
        "square picture drawn {w_sq} wide by {h_sq} tall (square units)"
    );
}

#[test]
fn a_wide_picture_is_centred_in_the_rows_it_does_not_fill() {
    let ps = paint(&bm(64, 16, |_, _| [10, 20, 30, 255]), 40, 40, ASPECT);
    let (x0, y0, x1, y1) = bounds(&ps);
    assert!(
        (x0 - (40.0 - (x1 - x0)) / 2.0).abs() < 0.01,
        "off-centre across"
    );
    assert!(
        (y0 - (40.0 - (y1 - y0)) / 2.0).abs() < 0.01,
        "off-centre down"
    );
}

/// A flat run is one rectangle, not one per sample. Without the merge a
/// screenshot is tens of thousands of quads a frame.
#[test]
fn a_flat_row_costs_one_quad_not_one_per_sample() {
    let ps = paint(&bm(64, 4, |_, _| [7, 7, 7, 255]), 60, 20, ASPECT);
    assert!(
        ps.len() <= 8,
        "a four-row flat picture took {} quads",
        ps.len()
    );
}

/// …and a picture that is genuinely different everywhere still gets drawn at
/// real detail: the merge must not be the only reason the count is small.
#[test]
fn a_detailed_picture_is_drawn_at_more_than_one_quad_per_cell() {
    let img = bm(120, 120, |x, y| {
        [(x * 2) as u8, (y * 3) as u8, (x ^ y) as u8, 255]
    });
    let ps = paint(&img, 40, 20, ASPECT);
    assert!(ps.len() > 40 * 20, "only {} quads for 800 cells", ps.len());
}

/// A logo's transparent ground must show the page, on whatever theme the page
/// happens to be — so those samples are dropped rather than composited.
#[test]
fn transparent_samples_are_not_drawn_at_all() {
    let half = bm(8, 8, |x, _| match x < 4 {
        true => [255, 0, 0, 255],
        false => [255, 0, 0, 0],
    });
    let ps = paint(&half, 20, 20, ASPECT);
    assert!(!ps.is_empty());
    let (x0, _, x1, _) = bounds(&ps);
    let full = paint(&bm(8, 8, |_, _| [255, 0, 0, 255]), 20, 20, ASPECT);
    let (fx0, _, fx1, _) = bounds(&full);
    assert!(
        (x0 - fx0).abs() < 0.01,
        "the drawn half starts where it should"
    );
    assert!(
        x1 < fx0 + (fx1 - fx0) * 0.6,
        "the transparent half was drawn: {x1} vs {fx1}"
    );
}

#[test]
fn a_pane_with_no_room_draws_nothing_rather_than_dividing_by_zero() {
    let img = bm(8, 8, |_, _| [1, 2, 3, 255]);
    assert!(paint(&img, 0, 10, ASPECT).is_empty());
    assert!(paint(&img, 10, 0, ASPECT).is_empty());
    assert!(paint(&img, 10, 10, 0.0).is_empty());
}

/// Decoding is the worker's job and downscaling is part of it: the pane must
/// never be handed forty megapixels to walk on the winit thread.
#[test]
fn a_decoded_picture_is_downscaled_but_remembers_its_real_size() {
    let mut buf = std::io::Cursor::new(Vec::new());
    let big = image::RgbaImage::from_fn(1600, 800, |x, y| {
        image::Rgba([(x % 256) as u8, (y % 256) as u8, 64, 255])
    });
    image::DynamicImage::ImageRgba8(big)
        .write_to(&mut buf, image::ImageFormat::Png)
        .expect("encode");
    let got = decode(buf.get_ref()).expect("decodes");
    assert_eq!(got.src, (1600, 800), "the caption's numbers are the file's");
    assert!(
        got.w <= MAX_SAMPLES && got.h <= MAX_SAMPLES,
        "not downscaled"
    );
    assert_eq!(got.px.len(), (got.w * got.h) as usize);
}

#[test]
fn bytes_that_are_not_a_picture_decode_to_nothing() {
    assert!(decode(b"not an image at all").is_none());
    assert!(decode(&[]).is_none());
}

/// The clip is a box, not a size, because a pane has rows a picture must not
/// enter at BOTH ends: the sticky heading band owns the first row, a live
/// search owns the last. Found by looking at a document scrolled so its
/// picture was half off the top — it was drawn over the band naming the
/// section it was in.
#[test]
fn a_picture_is_kept_out_of_the_rows_reserved_at_either_end() {
    let img = bm(40, 40, |_, _| [200, 60, 60, 255]);
    // A box spanning the whole pane, clipped to everything but the first and
    // last rows.
    let ps = paint_at(&img, 0.0, -3.0, 20.0, 20.0, ASPECT, (0.0, 1.0, 20.0, 19.0));
    assert!(!ps.is_empty(), "still drawn, just trimmed");
    let (_, y0, _, y1) = bounds(&ps);
    assert!(y0 >= 0.99, "entered the sticky row: {y0}");
    assert!(y1 <= 19.01, "entered the search row: {y1}");
}
