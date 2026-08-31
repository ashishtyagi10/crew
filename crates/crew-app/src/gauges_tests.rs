use super::*;

#[test]
fn fill_color_thresholds() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    assert_eq!(fill_color(0.5), crate::palette::accent());
    assert_eq!(fill_color(0.8), t.status_fg);
    assert_eq!(fill_color(0.95), t.ansi[9]);
    assert_eq!(track_color(), t.border_normal);
}

#[test]
fn gauge_50_pct_balanced() {
    let cells = gauge_cells("CPU ", 0.5, 0, 40);
    assert!(!cells.is_empty());
    let filled = cells.iter().filter(|c| c.c == '█').count();
    let track = cells.iter().filter(|c| c.c == '░').count();
    assert!((filled as i32 - track as i32).unsigned_abs() <= 1);
}

#[test]
fn gauge_0_pct_no_filled() {
    let cells = gauge_cells("CPU ", 0.0, 0, 40);
    assert_eq!(cells.iter().filter(|c| c.c == '█').count(), 0);
}

#[test]
fn gauge_100_pct_no_track() {
    let cells = gauge_cells("CPU ", 1.0, 0, 40);
    assert_eq!(cells.iter().filter(|c| c.c == '░').count(), 0);
}

#[test]
fn render_stats_legend_and_gauges() {
    let stats = Stats {
        cpu: 0.1,
        mem: 0.2,
        disk: 0.3,
        ..Default::default()
    };
    // A narrow nav keeps the bars.
    let cells = render_stats(stats, 16, 12, None);
    // flat divider, not a box
    assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
    assert!(!cells.iter().any(|c| matches!(c.c, '╭' | '╮' | '╰' | '╯')));
    // SYSTEM legend on the divider row
    assert!(cells.iter().any(|c| c.c == 'S' && c.row == 0));
    // gauge bars present, stacked on rows 1/2/3
    assert!(cells.iter().any(|c| c.c == '█' || c.c == '░'));
    let rows: std::collections::HashSet<u16> = cells.iter().map(|c| c.row).collect();
    assert!(rows.contains(&1) && rows.contains(&2) && rows.contains(&3));
}

/// The same section, wide: the readings become dials, and the bars they
/// replace leave no glyphs behind.
#[test]
fn a_wide_nav_draws_dials_instead_of_bars() {
    let _g = crate::app::theme_test_guard();
    let stats = Stats {
        cpu: 0.1,
        mem: 0.2,
        disk: 0.34,
        ..Default::default()
    };
    let cells = render_stats(stats, 24, 12, None);
    assert!(!cells.iter().any(|c| c.c == '█' || c.c == '░'), "no bars");
    let text = |r: u16| -> String {
        let mut v: Vec<_> = cells.iter().filter(|c| c.row == r).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    // Readings in the faces' windows on the block's third row, names on
    // its fourth.
    assert!(text(3).contains("10") && text(3).contains("20") && text(3).contains("34"));
    assert!(text(4).contains("cpu") && text(4).contains("mem") && text(4).contains("dsk"));
}

/// The tier mark has to reach the drawn row, and cost nothing when it is
/// not wanted: it rides the label's trailing space, so the bar and the
/// percentage must land on exactly the same columns either way. A cue
/// that shifted the layout would be a cue nobody could leave on.
#[test]
fn the_tier_mark_rides_the_label_space_without_moving_anything() {
    let _g = crate::app::motion_test_guard();
    let cols = 40;
    crate::shapecues::set(false);
    let off = gauge_cells("CPU ", 0.95, 0, cols);
    crate::shapecues::set(true);
    let on = gauge_cells("CPU ", 0.95, 0, cols);
    crate::shapecues::set(false);

    assert_eq!(off.len(), on.len(), "the cue must not change the width");
    let bar_and_pct = |v: &[CellView]| -> Vec<(u16, char)> {
        v.iter()
            .filter(|c| c.col > 4)
            .map(|c| (c.col, c.c))
            .collect()
    };
    assert_eq!(
        bar_and_pct(&off),
        bar_and_pct(&on),
        "the bar and the reading must not move"
    );

    let at = |v: &[CellView], col: u16| v.iter().find(|c| c.col == col).map(|c| c.c);
    assert_eq!(at(&off, 4), Some(' '), "off, the slot is the label space");
    assert_eq!(at(&on, 4), Some('\u{203c}'), "on, critical is marked");
}

/// Three bands, three appearances — a warning and a critical reading that
/// mark the same are no better than two that only differ in colour.
#[test]
fn each_band_marks_differently_on_a_drawn_row() {
    let _g = crate::app::motion_test_guard();
    crate::shapecues::set(true);
    let mark = |frac: f32| {
        gauge_cells("CPU ", frac, 0, 40)
            .iter()
            .find(|c| c.col == 4)
            .map(|c| c.c)
    };
    let (n, w, c) = (mark(0.3), mark(0.8), mark(0.95));
    crate::shapecues::set(false);
    assert_eq!(n, Some(' '), "nominal stays quiet");
    assert_ne!(w, n);
    assert_ne!(c, n);
    assert_ne!(w, c);
}

/// The CPU curve under the gauges is scaled to its own rolling peak, so
/// the rule says what that peak is. Without it the shape has no units.
#[test]
fn the_system_rule_names_the_curves_ceiling() {
    let _g = crate::app::theme_test_guard();
    let stats = Stats {
        cpu: 0.24,
        mem: 0.6,
        disk: 0.77,
        ..Default::default()
    };
    let rule = |peak| -> String {
        let mut v: Vec<_> = render_stats(stats, 28, 12, peak)
            .into_iter()
            .filter(|c| c.row == 0 && c.c != '─')
            .collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect::<String>().trim().to_string()
    };
    assert_eq!(rule(Some(47)), "SYSTEM peak 47%");
    // No history yet: the section is still itself, without a claim about a
    // ceiling it has not measured.
    assert_eq!(rule(None), "SYSTEM");
}
