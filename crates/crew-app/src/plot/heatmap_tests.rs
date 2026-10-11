use super::draw;
use crate::plot::Canvas;

fn shade(t: f32) -> ((u8, u8, u8), f32) {
    ((0, 200, 120), 0.08 + 0.92 * t)
}

fn grid(values: &[u64], rows: usize, cols: usize) -> Canvas {
    let mut c = Canvas::with_sub(24, 4, 2.0, 8);
    let (w, h) = c.size();
    draw(&mut c, (0.0, 0.0, w, h), values, rows, cols, 0.15, &shade);
    c
}

/// The strongest alpha painted at `(x, y)` in canvas units — a cell is
/// several rectangles once its edges are anti-aliased, so tests read the
/// grid by sampling it rather than by counting quads.
fn alpha_at(c: &Canvas, x: f32, y: f32) -> f32 {
    c.paint()
        .iter()
        .filter(|p| {
            let (py, ph) = (p.y * c.row_units(), p.h * c.row_units());
            x >= p.x && x < p.x + p.w && y >= py && y < py + ph
        })
        .map(|p| p.alpha)
        .fold(0.0f32, f32::max)
}

/// The colour painted at `(x, y)`, if any.
fn color_at(c: &Canvas, x: f32, y: f32) -> Option<(u8, u8, u8)> {
    c.paint()
        .iter()
        .find(|p| {
            let (py, ph) = (p.y * c.row_units(), p.h * c.row_units());
            x >= p.x && x < p.x + p.w && y >= py && y < py + ph
        })
        .map(|p| p.color)
}

/// Centre of cell `(row, col)` of an `rows`×`cols` grid on this canvas.
fn centre(c: &Canvas, rows: usize, cols: usize, r: usize, k: usize) -> (f32, f32) {
    let (w, h) = c.size();
    (
        (k as f32 + 0.5) * w / cols as f32,
        (r as f32 + 0.5) * h / rows as f32,
    )
}

#[test]
fn a_hotter_cell_is_drawn_stronger() {
    // The used hours: an idle one is a neutral dot, off the scale (below).
    let c = grid(&[0, 1, 5, 10], 1, 4);
    let a: Vec<f32> = (1..4)
        .map(|k| {
            let (x, y) = centre(&c, 1, 4, 0, k);
            alpha_at(&c, x, y)
        })
        .collect();
    assert!(
        a.windows(2).all(|w| w[0] <= w[1] + 1e-3),
        "alpha rises with value: {a:?}"
    );
    assert!(a[2] - a[0] > 0.5, "and by a lot: {a:?}");
}

#[test]
fn shading_is_relative_to_the_grids_own_peak() {
    // A quiet week and a busy one draw the same picture: the eye compares
    // within a heatmap, never between two of them.
    let quiet = grid(&[0, 1, 2, 4], 1, 4);
    let busy = grid(&[0, 1000, 2000, 4000], 1, 4);
    let sample = |c: &Canvas| -> Vec<f32> {
        (0..4)
            .map(|k| {
                let (x, y) = centre(c, 1, 4, 0, k);
                alpha_at(c, x, y)
            })
            .collect()
    };
    assert_eq!(sample(&quiet), sample(&busy));
}

#[test]
fn an_idle_grid_still_draws_its_cells() {
    // An idle week reads as an empty grid, not as a missing widget.
    let c = grid(&[0; 12], 3, 4);
    for r in 0..3 {
        for k in 0..4 {
            let (x, y) = centre(&c, 3, 4, r, k);
            assert!(alpha_at(&c, x, y) > 0.05, "cell ({r},{k}) is drawn");
        }
    }
}

#[test]
fn there_is_air_between_the_cells() {
    let c = grid(&[3; 24], 2, 12);
    let (w, h) = c.size();
    // On the boundary between two columns, nothing is painted.
    let x = w / 12.0;
    let y = h / 4.0;
    assert!(alpha_at(&c, x - 0.02, y) < 0.02, "the gap is empty");
    // …while a hair either side of it is inside a cell.
    assert!(alpha_at(&c, x + 0.1, y) > 0.05);
}

#[test]
fn every_cell_stays_inside_the_rect() {
    let c = grid(&[3; 24], 2, 12);
    let (w, h) = c.size();
    for p in c.paint() {
        assert!(p.x >= -1e-3 && p.x + p.w <= w + 1e-3, "{p:?}");
        let y = p.y * c.row_units();
        assert!(y >= -1e-3 && y + p.h * c.row_units() <= h + 1e-3, "{p:?}");
    }
}

#[test]
fn a_short_values_slice_leaves_the_rest_at_zero() {
    // Fewer readings than cells is normal (a week that started on
    // Wednesday); the missing ones are cold, not absent.
    let _g = crate::app::theme_test_guard();
    let c = grid(&[9, 9], 1, 4);
    let (x0, y0) = centre(&c, 1, 4, 0, 0);
    let (x3, y3) = centre(&c, 1, 4, 0, 3);
    assert!(alpha_at(&c, x0, y0) > 0.9, "the readings are hot");
    assert_eq!(
        color_at(&c, x3, y3),
        Some(super::idle_ink(&crew_theme::theme()).0),
        "the rest are idle dots"
    );
}

/// An idle hour is a dot at its cell's centre, a busy one fills its tile:
/// a quiet week reads as a calm dot grid, not a slab of 168 empty tiles.
#[test]
fn an_idle_cell_is_a_dot_and_a_busy_one_a_tile() {
    let c = grid(&[0, 9], 1, 2);
    let (w, h) = c.size();
    let near_edge = |k: usize| {
        let (x, y) = centre(&c, 1, 2, 0, k);
        alpha_at(&c, x - 0.3 * w / 2.0, y - 0.3 * h)
    };
    assert_eq!(near_edge(0), 0.0, "the idle cell's edge is open page");
    assert!(near_edge(1) > 0.5, "the busy cell fills its tile");
}

/// An idle hour is drawn as its idle dot, not as a reading on the heat
/// scale: a quiet day is a row of dots, not a row missing from the grid
/// (that it can be seen is the next test's).
#[test]
fn an_idle_row_is_dots_you_can_see() {
    let _g = crate::app::theme_test_guard();
    let (rows, cols) = (2, 6);
    let mut v = vec![0u64; rows * cols];
    v[3] = 50; // one busy hour on the first day; the second day is idle
    let c = grid(&v, rows, cols);
    let (x, y) = centre(&c, rows, cols, 1, 2);
    let (ink, alpha) = super::idle_ink(&crew_theme::theme());
    assert!(
        (alpha_at(&c, x, y) - alpha).abs() < 0.01,
        "idle dot at {}",
        alpha_at(&c, x, y)
    );
    assert_eq!(color_at(&c, x, y), Some(ink));
}

/// On glass the dot is a wash of the ink, so it reads over the frost
/// whatever desktop is under it — and stays quieter than any word. The
/// border's neutral measured 1.1:1 over a white desktop on four palettes,
/// and a quiet week vanished (glass survey D-L2).
#[test]
fn an_idle_dot_reads_over_the_glass_on_any_desktop() {
    let _g = crate::app::theme_test_guard();
    let over = |a: (u8, u8, u8), g: (u8, u8, u8), k: f32| {
        let m = |p: u8, q: u8| (f32::from(q) + (f32::from(p) - f32::from(q)) * k).round() as u8;
        (m(a.0, g.0), m(a.1, g.1), m(a.2, g.2))
    };
    let mut glass = 0;
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let t = crew_theme::theme();
        let (ink, alpha) = super::idle_ink(&t);
        for g in crew_theme::glasslegend::pane_grounds(&t) {
            let got = crew_theme::contrast_ratio(over(ink, g, alpha), g);
            assert!((1.8..=3.2).contains(&got), "{}: {got:.2}", id.as_str());
            glass += 1;
        }
    }
    assert!(glass >= 36, "the glass palettes were checked");
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}
