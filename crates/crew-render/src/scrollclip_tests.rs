use super::*;

fn q(y: f32, h: f32) -> Quad {
    Quad {
        x: 0.0,
        y,
        w: 10.0,
        h,
        color: [1.0; 4],
        radii: [3.0; 4],
    }
}

/// A shifted pane's quads move with its text and never leave its card: the
/// top row's background is cut at the card's top, the incoming row's at its
/// bottom, a quad wholly outside goes, and only cut edges lose their curve.
#[test]
fn quads_shift_and_are_cut_to_the_card() {
    // Card 100..200; rows of 20 px; shifted up 8 px.
    let mut quads = vec![q(100.0, 20.0), q(140.0, 20.0), q(200.0, 20.0), q(90.0, 5.0)];
    shift_and_clip(&mut quads, -8.0, 100.0, 200.0);
    assert_eq!(quads.len(), 3, "the quad above the card is gone");
    assert_eq!((quads[0].y, quads[0].h), (100.0, 12.0), "top row cut");
    assert_eq!(quads[0].radii, [0.0, 0.0, 3.0, 3.0]);
    assert_eq!(
        (quads[1].y, quads[1].h),
        (132.0, 20.0),
        "inside: just moved"
    );
    assert_eq!(quads[1].radii, [3.0; 4]);
    assert_eq!((quads[2].y, quads[2].h), (192.0, 8.0), "incoming row cut");
    assert_eq!(quads[2].radii, [3.0, 3.0, 0.0, 0.0]);
}

/// No shift, no change — the resting frame is the frame it always was.
#[test]
fn no_shift_touches_nothing() {
    let mut quads = vec![q(90.0, 30.0)];
    shift_and_clip(&mut quads, 0.0, 100.0, 200.0);
    assert_eq!((quads[0].y, quads[0].h), (90.0, 30.0));
    assert_eq!(layout_rows(10, 200.0, 20.0, 0.0), (10, 200.0));
    assert_eq!(layout_rows(10, 200.0, 20.0, -4.0), (11, 220.0));
}
