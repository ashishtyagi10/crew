use super::*;

fn text(cells: &[CellView], cols: usize) -> String {
    let mut line = vec![' '; cols];
    for c in cells {
        line[c.col as usize] = c.c;
    }
    line.into_iter().collect()
}

// ---- Pulse state ----

#[test]
fn record_hop_feeds_waterfall_and_history() {
    let mut p = Pulse::new();
    p.record_hop("planner", 2_000);
    p.record_hop("coder", 4_000);
    assert_eq!(
        p.hops(),
        &[("planner".to_string(), 2_000), ("coder".to_string(), 4_000)]
    );
    assert!(p.hist("planner").is_some());
    assert!(p.hist("reviewer").is_none());
}

#[test]
fn next_turn_resets_waterfall_but_keeps_history() {
    let mut p = Pulse::new();
    p.record_hop("planner", 2_000);
    p.end_turn();
    // The settled turn stays on screen until the next turn's first hop…
    assert_eq!(p.hops().len(), 1);
    p.begin_hop();
    assert!(p.hops().is_empty(), "new turn starts a fresh waterfall");
    assert!(
        p.hist("planner").is_some(),
        "sparkline history survives turns"
    );
}

#[test]
fn begin_hop_mid_turn_keeps_accumulating() {
    let mut p = Pulse::new();
    p.begin_hop();
    p.record_hop("planner", 1_000);
    p.begin_hop(); // next hop of the same turn
    p.record_hop("coder", 1_000);
    assert_eq!(p.hops().len(), 2);
}

// ---- Waterfall rendering ----

#[test]
fn waterfall_segments_are_proportional_with_gaps() {
    let hops = vec![("planner".to_string(), 6_000), ("coder".to_string(), 2_000)];
    let cells = waterfall_cells(80, 5, &hops, None);
    let planner = agent_color("planner");
    let coder = agent_color("coder");
    let pw = cells
        .iter()
        .filter(|c| c.c == '\u{2588}' && c.fg == planner)
        .count();
    let cw = cells
        .iter()
        .filter(|c| c.c == '\u{2588}' && c.fg == coder)
        .count();
    assert!(pw > cw * 2, "6s segment ≫ 2s segment (got {pw} vs {cw})");
    let line = text(&cells, 80);
    assert!(line.starts_with("turn "), "got: {line}");
    assert!(line.contains("8.0s"), "total label, got: {line}");
    assert!(!line.contains("\u{25b6}"), "no live marker when settled");
}

#[test]
fn waterfall_live_segment_and_marker() {
    let hops = vec![("planner".to_string(), 3_000)];
    let cells = waterfall_cells(80, 5, &hops, Some(("coder", 1_000)));
    let line = text(&cells, 80);
    assert!(line.contains("\u{25b6}"), "live marker, got: {line}");
    assert!(line.contains("4.0s"), "total includes live, got: {line}");
    let coder = agent_color("coder");
    assert!(
        cells.iter().any(|c| c.c == '\u{2588}' && c.fg == coder),
        "live segment drawn in agent colour"
    );
}

#[test]
fn waterfall_empty_without_hops_or_room() {
    assert!(waterfall_cells(80, 5, &[], None).is_empty());
    let hops = vec![("planner".to_string(), 1_000)];
    assert!(waterfall_cells(20, 5, &hops, None).is_empty(), "too narrow");
    let cells = waterfall_cells(80, 5, &hops, None);
    assert!(cells.iter().all(|c| c.col < 80));
}
