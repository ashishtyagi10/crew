use super::draw;
use crate::plot::Canvas;

/// Ink (alpha × area, in canvas units²) in each of `n` equal slots.
fn ink_per_slot(c: &Canvas, n: usize) -> Vec<f32> {
    let (w, _) = c.size();
    let slot = w / n as f32;
    let mut out = vec![0.0; n];
    for p in c.paint() {
        let i = ((p.x + p.w / 2.0) / slot) as usize;
        if let Some(s) = out.get_mut(i) {
            *s += p.w * p.h * c.row_units() * p.alpha;
        }
    }
    out
}

#[test]
fn a_taller_value_draws_a_taller_bar() {
    let mut c = Canvas::new(28, 4, 2.0);
    let (w, h) = c.size();
    draw(
        &mut c,
        (0.0, 0.0, w, h),
        &[0.25, 1.0, 0.5, 0.75],
        (255, 200, 0),
    );
    let ink = ink_per_slot(&c, 4);
    assert!(
        ink[1] > ink[3] && ink[3] > ink[2] && ink[2] > ink[0],
        "{ink:?}"
    );
}

#[test]
fn a_zero_day_is_a_stub_not_a_hole() {
    let mut c = Canvas::new(21, 3, 2.0);
    let (w, h) = c.size();
    draw(&mut c, (0.0, 0.0, w, h), &[1.0, 0.0, 1.0], (255, 200, 0));
    let ink = ink_per_slot(&c, 3);
    assert!(ink[1] > 0.0, "the zero day drew nothing");
    assert!(ink[1] < ink[0] * 0.1, "the stub is not a bar: {ink:?}");
}

#[test]
fn neighbouring_bars_do_not_touch() {
    // Every day its own bar: some column between two slots stays empty.
    let mut c = Canvas::new(14, 2, 2.0);
    let (w, h) = c.size();
    draw(&mut c, (0.0, 0.0, w, h), &[1.0, 1.0], (255, 200, 0));
    let mid = w / 2.0;
    assert!(
        c.paint()
            .iter()
            .all(|p| p.x >= mid || p.x + p.w <= mid + 1e-3),
        "a bar spans the boundary between two days"
    );
}

#[test]
fn nothing_to_draw_draws_nothing() {
    let mut c = Canvas::new(10, 2, 2.0);
    let (w, h) = c.size();
    draw(&mut c, (0.0, 0.0, w, h), &[], (255, 200, 0));
    assert!(c.paint().is_empty());
}
