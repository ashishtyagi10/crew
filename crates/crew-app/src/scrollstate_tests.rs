use super::*;

/// Every frame's rows until the glide stops, at 60 fps.
fn run(g: &mut ScrollGlide) -> Vec<i32> {
    let mut steps = Vec::new();
    while g.moving() {
        steps.push(g.step(16));
        assert!(steps.len() < 120, "a scroll glide must settle: {steps:?}");
    }
    steps
}

/// A page travels — over several frames, never backwards, never more than
/// asked — and arrives exactly, in about the third of a second it promises.
#[test]
fn a_page_travels_and_lands_exactly() {
    let mut g = ScrollGlide::default();
    g.push(40);
    let steps = run(&mut g);
    assert_eq!(steps.iter().sum::<i32>(), 40, "{steps:?}");
    assert!(steps.iter().all(|&d| d >= 0), "never backwards: {steps:?}");
    assert!(
        steps.iter().filter(|&&d| d != 0).count() > 4,
        "a glide, not a jump: {steps:?}"
    );
    // The rows are all there in ~300 ms; the last few frames are the drawn
    // remainder settling to under a pixel.
    let rows_done = steps.iter().rposition(|&d| d != 0).unwrap() + 1;
    assert!(rows_done * 16 <= 400, "too slow: {} ms", rows_done * 16);
    assert!(steps.len() * 16 <= 500, "settles: {} ms", steps.len() * 16);
    // It starts gently: the first frame is a few rows, not most of the page.
    assert!(steps[0] < 20, "{steps:?}");
}

/// Where the drawing is, in rows: the rows handed over plus the remainder.
fn drawn(g: &ScrollGlide) -> f32 {
    g.applied() as f32 + g.residual()
}

fn pixel() -> ScrollGlide {
    ScrollGlide {
        pixel: true,
        ..Default::default()
    }
}

/// A pane that draws only whole rows (chat, viewer, todo): down is the exact
/// mirror of up, row for row — neither direction holds its last row back.
#[test]
fn whole_rows_mirror_exactly() {
    for n in [1, 3, 25] {
        let mut up = ScrollGlide::default();
        up.push(n);
        let mut down = ScrollGlide::default();
        down.push(-n);
        let neg: Vec<i32> = run(&mut down).iter().map(|d| -d).collect();
        assert_eq!(run(&mut up), neg, "{n} rows");
        assert_eq!(up.residual(), 0.0, "nothing drawn between rows");
    }
}

/// A terminal: down mirrors up in what is DRAWN. The rows handed over do
/// not mirror (both take the row at or above the spring), which is the
/// point — the remainder is always drawn by sliding up, never down.
#[test]
fn drawn_between_rows_down_mirrors_up() {
    let mut up = pixel();
    up.push(25);
    let mut down = pixel();
    down.push(-25);
    while up.moving() || down.moving() {
        up.step(16);
        down.step(16);
        assert!(
            (drawn(&up) + drawn(&down)).abs() < 0.01,
            "{} vs {}",
            drawn(&up),
            drawn(&down)
        );
        assert!(up.residual() <= 0.0 && up.residual() > -1.0);
        assert!(down.residual() <= 0.0 && down.residual() > -1.0);
    }
    assert_eq!((up.applied(), down.applied()), (0, 0), "both stopped");
}

/// The drawn position moves smoothly: no frame jumps more than a few rows,
/// and between frames it never runs backwards.
#[test]
fn the_drawn_position_is_continuous() {
    let mut g = pixel();
    g.push(10);
    let mut last = 0.0;
    let mut total = 0;
    while g.moving() {
        total += g.step(16);
        let now = total as f32 + g.residual();
        assert!(now >= last - 0.001, "backwards: {last} → {now}");
        assert!(now - last < 3.0, "a jump: {last} → {now}");
        last = now;
    }
    assert_eq!(total, 10);
}

/// A second notch mid-flight adds to the trip and keeps the speed: the sum is
/// both notches, and the motion never pauses or reverses at the seam.
#[test]
fn a_second_notch_mid_flight_extends_the_same_motion() {
    let mut g = ScrollGlide::default();
    g.push(3);
    let first = g.step(16);
    g.push(3);
    let rest = run(&mut g);
    assert_eq!(first + rest.iter().sum::<i32>(), 6);
    assert!(rest.iter().all(|&d| d >= 0), "{rest:?}");
    // And the reverse notch turns it around rather than queueing behind it.
    let mut g = ScrollGlide::default();
    g.push(9);
    let went = g.step(16) + g.step(16);
    g.push(-9);
    let back: i32 = run(&mut g).iter().sum();
    assert_eq!(went + back, 0);
}

/// Stopping drops what is owed, so the redraw registry goes quiet.
#[test]
fn stop_owes_nothing() {
    let mut g = ScrollGlide::default();
    g.push(12);
    assert!(g.moving());
    g.stop();
    assert!(!g.moving());
    assert_eq!(g.step(16), 0);
}
