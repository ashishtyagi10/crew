use super::*;

fn stats() -> Stats {
    Stats {
        cpu: 0.24,
        mem: 0.60,
        disk: 0.77,
        ..Default::default()
    }
}

/// Dragged wide, the rings used to stay pinned at the left with a third
/// of the section empty beside them. They spread — up to MAX_SLOT — and
/// then the group centres in whatever is left, so they keep reading as
/// one answer in three parts instead of three unrelated dials.
#[test]
fn the_dials_spread_then_centre_instead_of_hugging_the_left_edge() {
    let span = |cols| NAV.centre_x(2, cols) - NAV.centre_x(0, cols);
    assert!(span(37) > span(21), "{} vs {}", span(37), span(21));
    assert!(
        (span(80) - span(37)).abs() < 1e-3,
        "and stop spreading at the cap: {} vs {}",
        span(80),
        span(37)
    );
    // Centred: the air left of the first face matches the air right of
    // the last, within a column.
    let cols = 60u16;
    let r = NAV.radius(cols, 2.0);
    let left = NAV.centre_x(0, cols) - r - f32::from(crate::navtext::INDENT);
    let right = f32::from(cols - 1) - (NAV.centre_x(2, cols) + r);
    assert!((left - right).abs() < 1.5, "left {left}, right {right}");
}

/// What the block costs the frame. Every rectangle here is a quad the
/// GPU draws on top of the ~1500 the cell backgrounds already push, and
/// this section redraws on the sampler's every second — so the number is
/// worth knowing rather than assuming. Three faces at sixteen canvas
/// pixels to the column is the resolution that stopped the arc
/// staircasing; if it ever costs thousands of quads, the resolution is
/// the knob to turn, not the shapes.
#[test]
fn the_three_faces_cost_the_frame_a_bounded_number_of_quads() {
    let _g = crate::app::theme_test_guard();
    for cols in [MIN_COLS, 26, 37, 60] {
        let n = NAV.paint(stats(), cols, 1, 2.0).len();
        assert!(n < 1600, "cols={cols}: {n} quads");
    }
}

/// The scale has to be visible on the page it is drawn on, or the dial is
/// a needle pointing at nothing. The palette's own border shade is not:
/// on the light themes it lands near 1.2 against the page, which is what
/// the first light-theme shot of this section showed — three faces with a
/// hand and no marks. Both scale colours are derived against the page,
/// and both are laid down opaque, so what is measured here is what is
/// drawn.
#[test]
fn the_scale_reads_on_every_page() {
    let _g = crate::app::theme_test_guard();
    let floor = crew_theme::contrast::mark_floor();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let page = crew_theme::theme().page_bg;
        let (major, minor) = scale_colors();
        for (what, c) in [("major", major), ("minor", minor)] {
            let cr = crew_theme::contrast_ratio(c, page);
            assert!(cr >= floor - 0.01, "{id:?} {what} tick at {cr:.2}");
        }
        // …and the ranking survives: a minor tick is never louder than a
        // major one, however little headroom the page leaves.
        assert!(
            crew_theme::contrast_ratio(minor, page)
                <= crew_theme::contrast_ratio(major, page) + 0.01,
            "{id:?} minor ticks outshout the majors"
        );
    }
}

/// …and at the narrowest nav that still gets dials they sit where they
/// always did: indented under the legend, one slot each, nothing wasted.
#[test]
fn the_narrowest_dial_nav_keeps_the_slot_it_was_built_for() {
    assert!(fits(MIN_COLS) && !fits(MIN_COLS - 1));
    let gap = NAV.centre_x(1, MIN_COLS) - NAV.centre_x(0, MIN_COLS);
    assert!((gap - f32::from(SLOT)).abs() < 1e-3, "gap {gap}");
    assert!(NAV.centre_x(0, MIN_COLS) >= f32::from(crate::navtext::INDENT));
}

/// The reading lands in the gap of its own face at every width and in
/// both blocks — the number and the needle are drawn by two different
/// passes off one `centre_x`, and the dashboard's block is two rows
/// taller, so the row the digits go on is derived rather than counted.
#[test]
fn every_reading_sits_in_its_own_face() {
    let _g = crate::app::theme_test_guard();
    for (d, widths) in [(NAV, &[MIN_COLS, 26, 37, 60][..]), (DASH, &[DASH_COLS][..])] {
        for &cols in widths {
            let cells = d.cells(stats(), cols, 0);
            let digits = d.digits_row(cols);
            for (i, want) in ["24", "60", "77"].into_iter().enumerate() {
                let cx = d.centre_x(i, cols);
                let text: String = {
                    let mut v: Vec<_> = cells
                        .iter()
                        .filter(|c| {
                            c.row == digits && (f32::from(c.col) - cx).abs() <= d.radius(cols, 2.0)
                        })
                        .collect();
                    v.sort_by_key(|c| c.col);
                    v.iter().map(|c| c.c).collect()
                };
                assert_eq!(text, want, "{d:?} cols={cols} dial {i}");
            }
            // …and the names are on the block's last row, under them.
            assert!(cells.iter().any(|c| c.row == d.rows - 1 && c.c == 'c'));
        }
    }
}

/// Whatever the block's shape, the number is a window *in* the face: far
/// enough below the hub to clear the hand, inside the rim, and never on a
/// row the scale's own ticks reach.
#[test]
fn the_number_stays_inside_the_face_at_every_block_size() {
    for d in [NAV, DASH] {
        for cols in [MIN_COLS, 26, 30, 36, 60, 200] {
            let r = d.radius(cols, 2.0);
            // Where the digit row's ink sits, relative to the centre.
            let below = (f32::from(d.digits_row(cols)) + 0.5) * 2.0 - d.cy(2.0);
            assert!(below > r * 0.35, "{d:?}/{cols}: {below} crowds the hub");
            assert!(below < r * 0.92, "{d:?}/{cols}: {below} is off the face");
        }
    }
}

/// The dashboard exists to show these widgets at a size the nav cannot.
/// It has the rows; the faces have to actually use them.
#[test]
fn the_dashboards_faces_are_bigger_than_any_the_nav_can_draw() {
    let nav = NAV.radius(200, 2.0); // the nav at its very widest
    let dash = DASH.radius(DASH_COLS, 2.0);
    assert!(dash > nav * 1.5, "dash {dash} vs nav {nav}");
    // Neither may be taller than the rows it was given, or the section
    // above would clip its scale.
    for (d, cols) in [(NAV, 200u16), (DASH, DASH_COLS)] {
        assert!(
            d.radius(cols, 2.0) <= d.cy(2.0) + 1e-3,
            "{d:?} face overflows its rows"
        );
    }
}
