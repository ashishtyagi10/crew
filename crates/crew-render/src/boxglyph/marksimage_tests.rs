//! `▣`, crew's image mark: a frame with a picture in it, drawn.
use crate::boxglyph::synth;

#[test]
fn the_image_mark_is_a_frame_around_a_filled_centre() {
    let img = synth('\u{25A3}', 10, 20, 12).expect("claimed and drawn");
    let (w, h) = (img.placement.width, img.placement.height);
    let at = |x: u32, y: u32| img.data[(y * w + x) as usize];
    let (cx, cy) = (w / 2, h / 2);
    assert!(at(cx, cy) > 200, "the picture in the middle is filled");
    // Between the picture and the frame: the gap that makes it a frame.
    let ring = (1..cx).map(|d| at(cx - d, cy)).collect::<Vec<_>>();
    let gap = ring.iter().position(|&v| v < 40).expect("a gap");
    assert!(
        ring[gap..].iter().any(|&v| v > 200),
        "and the frame beyond it: {ring:?}"
    );
}
