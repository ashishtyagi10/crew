use super::*;

/// The hover promises exactly what the drawing already promised: the URL
/// spans `linkhl` tints, and the file references `pathhl` rules.
#[test]
fn the_span_is_the_run_that_is_drawn_as_a_link() {
    let line = "see https://example.com/a and src/main.rs for it";
    let url = line.find("https").unwrap();
    let path = line.find("src/").unwrap();
    // Anywhere inside the URL answers with the whole URL.
    for col in url..url + 5 {
        assert_eq!(
            span_at(line, col),
            Some((url, url + "https://example.com/a".len()))
        );
    }
    assert_eq!(
        span_at(line, path + 2),
        Some((path, path + "src/main.rs".len()))
    );
    // Prose is not a link.
    assert_eq!(span_at(line, 0), None, "\"see\" is not a link");
    assert_eq!(span_at(line, 3), None, "nor is the space after it");
    assert_eq!(span_at(line, line.len() - 1), None);
}

/// A URL whose path looks like a file reference is a URL, not a path — the
/// two matchers overlap there and the answer has to be one of them.
#[test]
fn a_url_containing_a_path_is_still_one_run() {
    let line = "https://example.com/src/main.rs";
    assert_eq!(span_at(line, 25), Some((0, line.len())));
}

#[test]
fn nothing_under_the_pointer_is_nothing() {
    assert_eq!(span_at("", 0), None);
    assert_eq!(span_at("plain prose here", 4), None);
    assert_eq!(
        span_at("https://x.dev", 99),
        None,
        "past the end of the line"
    );
}

/// Exactly one run on the canvas ever lights: the published hover names a
/// pane, and every other pane reads `None` from it.
#[test]
fn only_the_hovered_pane_reads_a_run() {
    let _g = test_guard();
    publish(Some((3, 7, 2, 9)));
    assert_eq!(for_pane(3), Some((7, 2, 9)));
    assert_eq!(for_pane(0), None);
    assert_eq!(for_pane(4), None);
    assert!(any());
    publish(None);
    assert_eq!(for_pane(3), None);
    assert!(!any());
}

/// Pane 0 is a real pane, and the encoding must not mistake it for "none" —
/// which is why the index is stored one-based.
#[test]
fn pane_zero_is_a_pane_and_not_an_absence() {
    let _g = test_guard();
    publish(Some((0, 0, 0, 4)));
    assert_eq!(for_pane(0), Some((0, 0, 4)));
    assert!(any());
    publish(None);
}

/// The publish reports whether it MOVED, because the hovered run's weight is
/// part of the frame — a hover that changed nothing must not cost a repaint.
#[test]
fn publishing_reports_only_real_movement() {
    let _g = test_guard();
    publish(None);
    assert!(publish(Some((1, 2, 3, 4))), "arriving is a change");
    assert!(!publish(Some((1, 2, 3, 4))), "staying put is not");
    assert!(publish(Some((1, 2, 3, 5))), "a different run is");
    assert!(publish(None), "leaving is");
    assert!(!publish(None));
}

/// A value that cannot be encoded publishes NOTHING rather than aliasing onto
/// another pane's run — a wrong run lit on a wrong pane is worse than none.
#[test]
fn an_unencodable_run_publishes_nothing() {
    let _g = test_guard();
    publish(None);
    assert!(!publish(Some((0, 0, 0, 70_000))), "a column past 16 bits");
    assert!(!any());
    assert!(!publish(Some((usize::MAX, 0, 0, 1))), "a pane past 16 bits");
    assert!(!any());
}

#[test]
fn marking_emboldens_the_run_and_nothing_else() {
    let _g = test_guard();
    let cell = |col: u16, row: u16| CellView {
        col,
        row,
        c: 'x',
        ..Default::default()
    };
    let mut cells: Vec<CellView> = (0..6)
        .flat_map(|r| (0..6).map(move |c| cell(c, r)))
        .collect();
    publish(Some((1, 2, 1, 4)));
    mark(&mut cells, 1);
    for c in &cells {
        let inside = c.row == 2 && (1..4).contains(&c.col);
        assert_eq!(c.bold, inside, "({}, {}) bold={}", c.col, c.row, c.bold);
    }
    // A different pane's cells are left alone entirely.
    let mut other: Vec<CellView> = vec![cell(2, 2)];
    mark(&mut other, 0);
    assert!(!other[0].bold);
    publish(None);
}
