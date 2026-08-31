use super::Paint;

#[test]
fn sub_pixel_and_transparent_paint_is_dropped() {
    assert!(Paint::solid(0.0, 0.0, 1.0, 1.0, (255, 0, 0)).visible());
    assert!(!Paint::solid(0.0, 0.0, 0.0, 1.0, (255, 0, 0)).visible());
    assert!(!Paint::solid(0.0, 0.0, 1.0, 1.0, (255, 0, 0))
        .at(0.0)
        .visible());
    // A rasterizer emits a great many near-zero-alpha edge pixels; they
    // cost a quad each and put nothing on the screen.
    assert!(!Paint::solid(0.0, 0.0, 1.0, 1.0, (255, 0, 0))
        .at(0.001)
        .visible());
}

#[test]
fn shifting_moves_the_origin_and_nothing_else() {
    let p = Paint::solid(1.0, 2.0, 3.0, 4.0, (1, 2, 3))
        .at(0.5)
        .shifted(10.0, 20.0);
    assert_eq!((p.x, p.y, p.w, p.h), (11.0, 22.0, 3.0, 4.0));
    assert_eq!(p.alpha, 0.5);
}
