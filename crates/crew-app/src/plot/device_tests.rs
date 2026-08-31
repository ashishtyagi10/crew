use super::*;

/// The whole point: the raster follows the device, so the same widget is
/// drawn at 8 canvas pixels per column on a 1× display and 16 on a
/// Retina one — never at a constant that is right for neither.
#[test]
fn the_raster_follows_the_cell() {
    set_cell_w(8.0);
    assert_eq!(sub(), 8);
    set_cell_w(16.0);
    assert_eq!(sub(), 16, "a Retina cell gets a Retina raster");
    set_cell_w(0.0);
    assert_eq!(sub(), FALLBACK, "no frame yet is not a zero-wide canvas");
}

#[test]
fn absurd_cells_are_clamped_rather_than_believed() {
    set_cell_w(1.0);
    assert_eq!(sub(), FLOOR);
    set_cell_w(400.0);
    assert_eq!(sub(), CEIL);
    set_cell_w(f32::NAN);
    assert_eq!(sub(), FALLBACK);
    set_cell_w(8.0);
}
