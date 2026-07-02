use std::collections::HashMap;
use std::time::Instant;

use super::*;

fn agent(name: &str) -> AgentInfo {
    AgentInfo {
        name: name.into(),
        role: String::new(),
        model: "qwen-max".into(),
    }
}

fn active(name: &str) -> ActiveAgent {
    ActiveAgent {
        name: name.into(),
        from: "user".into(),
        since: Instant::now(),
    }
}

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
    assert!(p.is_empty());
    p.record_hop("planner", 2_000);
    p.record_hop("coder", 4_000);
    assert_eq!(
        p.hops(),
        &[("planner".to_string(), 2_000), ("coder".to_string(), 4_000)]
    );
    assert!(p.hist("planner").is_some());
    assert!(p.hist("reviewer").is_none());
    assert!(!p.is_empty());
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

// ---- Lane gating ----

#[test]
fn lanes_need_height_agents_and_engagement() {
    assert_eq!(pulse_lanes(3, 20, true), 3);
    assert_eq!(pulse_lanes(3, 13, true), 0, "too short");
    assert_eq!(pulse_lanes(0, 20, true), 0, "no agents");
    assert_eq!(pulse_lanes(3, 20, false), 0, "not engaged");
    assert_eq!(pulse_lanes(9, 20, true), 6, "lanes cap at 6");
}

// ---- Lane rendering ----

fn lane_ctx(
    cols: u16,
    active_agent: Option<&ActiveAgent>,
    stats_ms: u64,
    ctx: Option<u64>,
    limit: Option<u64>,
) -> Vec<CellView> {
    let mut stats = HashMap::new();
    stats.insert("planner".to_string(), (2u32, stats_ms));
    let mut hist = History::new(8);
    hist.push(1_000);
    hist.push(3_000);
    lane_cells(
        cols,
        1,
        &agent("planner"),
        active_agent,
        Some(&hist),
        &stats,
        10_000,
        3_000,
        7,
        ctx,
        limit,
    )
}

fn lane(cols: u16, active_agent: Option<&ActiveAgent>, stats_ms: u64) -> Vec<CellView> {
    lane_ctx(cols, active_agent, stats_ms, None, None)
}

#[test]
fn idle_lane_shows_marker_name_and_stat() {
    let line = text(&lane(80, None, 6_400), 80);
    assert!(line.contains("\u{25aa} planner"), "got: {line}");
    assert!(
        line.contains("\u{00b7}2\u{00d7} 3.2s"),
        "idle stat, got: {line}"
    );
}

#[test]
fn active_lane_shows_spinner_elapsed_and_bold_name() {
    let a = active("planner");
    let cells = lane(80, Some(&a), 6_400);
    let line = text(&cells, 80);
    assert!(line.contains("\u{25b8} planner"), "got: {line}");
    assert!(line.contains(" 0s"), "live elapsed, got: {line}");
    assert!(cells.iter().any(|c| c.bold), "active name is bold");
}

#[test]
fn lane_has_sparkline_in_agent_color_and_share_bar() {
    let cells = lane(80, None, 6_400);
    let color = agent_color("planner");
    // Sparkline: block-ramp glyphs in the agent colour.
    assert!(
        cells
            .iter()
            .any(|c| ('\u{2581}'..='\u{2588}').contains(&c.c) && c.fg == color),
        "sparkline cells present"
    );
    // Share bar (the last 15 columns): 64% of 10 cells → 6 filled `█` +
    // 4 track `░`, then the %.
    let bar = |ch: char| {
        cells
            .iter()
            .filter(|c| c.c == ch && c.col >= 80 - 15)
            .count()
    };
    assert_eq!((bar('\u{2588}'), bar('\u{2591}')), (6, 4), "64% share bar");
    assert!(text(&cells, 80).contains("64%"));
}

#[test]
fn narrow_lane_drops_charts_but_keeps_identity() {
    let cells = lane(30, None, 6_400);
    let line = text(&cells, 30);
    assert!(line.contains("planner"), "got: {line}");
    assert!(!line.contains('\u{2591}'), "no bar track at 30 cols");
    assert!(cells.iter().all(|c| c.col < 30), "clipped to width");
}

// ---- Context meter ----

#[test]
fn ctx_meter_shows_fill_percent_against_the_limit() {
    // 8 192 of 32 768 → 25%: 1 of 5 meter cells filled, accent tier.
    let cells = lane_ctx(100, None, 6_400, Some(8_192), Some(32_768));
    let line = text(&cells, 100);
    assert!(line.contains("ctx"), "got: {line}");
    assert!(line.contains("25%"), "got: {line}");
    let meter: Vec<_> = cells
        .iter()
        .filter(|c| c.c == '\u{2588}' && c.fg == crate::palette::accent())
        .collect();
    assert_eq!(meter.len(), 1, "25% of a 5-cell meter fills one cell");
}

#[test]
fn ctx_meter_goes_red_when_the_window_is_nearly_full() {
    let cells = lane_ctx(100, None, 6_400, Some(31_000), Some(32_768));
    let line = text(&cells, 100);
    assert!(line.contains("95%"), "got: {line}");
    let red = crew_theme::theme().ansi[9];
    assert!(
        cells.iter().any(|c| c.c == '\u{2588}' && c.fg == red),
        "past 90% the fill wears the critical tier"
    );
}

#[test]
fn ctx_meter_without_limit_shows_absolute_tokens() {
    let line = text(&lane_ctx(100, None, 6_400, Some(8_192), None), 100);
    assert!(line.contains("ctx 8.2k"), "got: {line}");
    assert!(
        !line.contains('%') || line.contains("64%"),
        "no ctx percent, got: {line}"
    );
}

#[test]
fn ctx_meter_hidden_without_data_or_width() {
    assert!(!text(&lane(100, None, 6_400), 100).contains("ctx"));
    let narrow = text(&lane_ctx(70, None, 6_400, Some(8_192), Some(32_768)), 70);
    assert!(!narrow.contains("ctx"), "no room at 70 cols, got: {narrow}");
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
