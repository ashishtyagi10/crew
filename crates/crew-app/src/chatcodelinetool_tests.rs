//! A tool that runs quietly is work, not a stall: Claude Code prints nothing
//! while its `cargo test` runs, and the line dimmed and crawled for the
//! whole run while the tool line under it counted the seconds up.
use crate::chatcodeline::tests::frame;
use crate::chatcodeline::{cells_at, Frame, INSET, STALL_MS};

#[test]
fn a_quiet_running_tool_does_not_dim_the_line() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    let dist = |a: (u8, u8, u8)| {
        let d = |x: u8, y: u8| (x as i32 - y as i32).abs();
        d(a.0, t.ink.0) + d(a.1, t.ink.1) + d(a.2, t.ink.2)
    };
    let now = 1_000_000 + 60_000;
    let quiet = now - STALL_MS - 30_000;
    let first = |f: &Frame| cells_at(f, now).iter().find(|c| c.col == INSET).unwrap().fg;
    let mut running = frame(quiet);
    running.tool = true;
    assert!(
        dist(first(&running)) < dist(first(&frame(quiet))),
        "the stalled line is the dimmer one"
    );
}
