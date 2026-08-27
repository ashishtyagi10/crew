use super::*;

#[test]
fn every_level_round_trips_and_has_synonyms() {
    for l in Leading::ALL {
        assert_eq!(Leading::parse(l.as_str()), Some(l), "{}", l.as_str());
    }
    assert_eq!(Leading::parse(" TIGHT "), Some(Leading::Tight));
    assert_eq!(Leading::parse("default"), Some(Leading::Normal));
    assert_eq!(Leading::parse("airy"), Some(Leading::Loose));
    assert_eq!(Leading::parse("enormous"), None);
}

/// Choosing the default must change nothing: `normal` is exactly the ratio
/// crew has always drawn, taken from the renderer's own constant rather than
/// written down a second time.
#[test]
fn normal_is_the_ratio_crew_has_always_drawn() {
    assert_eq!(Leading::Normal.ratio(), crew_render::CELL_H_RATIO);
    assert_eq!(Leading::Normal.ratio(), 1.25);
}

/// The ladder only ever goes up, and it stays inside the bounds the doc
/// comment argues for: below ~1.1 a monospace face's descenders meet the
/// ascenders of the row beneath, and above ~1.65 the cell is a stripe with
/// the text loose inside it (the cell is also the cursor and the selection).
#[test]
fn the_ladder_rises_and_stays_inside_its_bounds() {
    let ratios: Vec<f32> = Leading::ALL.iter().map(|l| l.ratio()).collect();
    for pair in ratios.windows(2) {
        assert!(pair[1] > pair[0], "{ratios:?} is not a ladder");
    }
    assert!(ratios[0] >= 1.10, "rows set solid would collide");
    assert!(*ratios.last().unwrap() <= 1.65, "a stripe, not a line");
}

/// A row's height is the font size times the leading, and it is the SAME
/// product the renderer's cell box takes — the window-sizing math and the
/// cell grid cannot be allowed to disagree about how tall a row is.
#[test]
fn the_config_row_height_is_the_font_size_times_the_leading() {
    let mut cfg = crate::config::CrewConfig {
        font_size: 20.0,
        ..Default::default()
    };
    assert!((cfg.line_height() - 25.0).abs() < 1e-6, "the default");
    for l in Leading::ALL {
        cfg.leading = l.as_str().to_string();
        assert!((cfg.line_height() - 20.0 * l.ratio()).abs() < 1e-6, "{l:?}");
    }
    // A typo must not silently re-space every line of every pane.
    cfg.leading = "enormous".into();
    assert_eq!(cfg.leading(), Leading::Normal);
}
