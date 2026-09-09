//! The pane-level path: hive events in through `absorb_hive`, rows out of
//! the card view, a click on the summary line and on a call's line.
use crate::chat::tests::pane;
use crate::chat::ChatPane;
use crate::chatmsgs::card_lines;
use crate::chatmsgs::tests::msg;
use crate::motion::set_level;
use crate::motion::MotionLevel::Full;
use crew_hive::{AgentId, HiveEvent};

const COLS: u16 = 60;
const ROWS: u16 = 30;

fn rows(p: &ChatPane) -> Vec<String> {
    card_lines(&p.visible_messages(), COLS as usize, 0, p.view())
        .iter()
        .map(|l| l.iter().map(|c| c.c).collect())
        .collect()
}

/// The absolute pane row whose text contains `needle`.
fn row_of(p: &ChatPane, needle: &str) -> u16 {
    crate::chatplace::placed_lines(p, COLS, ROWS)
        .iter()
        .find(|(_, l)| l.iter().map(|c| c.c).collect::<String>().contains(needle))
        .map(|(r, _)| *r)
        .unwrap_or_else(|| panic!("no row with {needle:?}: {:?}", rows(p)))
}

/// A pane mid-swarm: a plan note in the transcript, agent 7 named `coder`
/// by the broker, one call in flight.
fn calling() -> ChatPane {
    let mut p = pane();
    p.push_capped(msg("crew", "plan: read the file"));
    p.absorb_hive(&HiveEvent::ToolCall {
        agent: AgentId(7),
        label: "fs:read".into(),
        args: r#"{"path":"src/foo.rs"}"#.into(),
    });
    p.tools.bind("coder");
    p
}

fn finish(p: &mut ChatPane) {
    p.absorb_hive(&HiveEvent::ToolResult {
        agent: AgentId(7),
        label: "fs:read".into(),
        ok: true,
        text: "use std;\nfn main() {}".into(),
        ms: 120,
    });
}

#[test]
fn a_call_draws_a_pending_line_and_its_result_a_done_one() {
    let _g = crate::app::motion_test_guard();
    set_level(Full);
    let _f = crate::glyphs::force(false);
    let mut p = calling();
    let r = rows(&p);
    let line = r.last().unwrap();
    assert!(
        line.starts_with("  | fs:read src/foo.rs")
            || line.starts_with("  / fs:read src/foo.rs")
            || line.starts_with("  - fs:read src/foo.rs")
            || line.starts_with("  \\ fs:read src/foo.rs"),
        "a spinning line under the agent's thin card: {r:?}"
    );
    assert!(
        r[r.len() - 2].contains("coder"),
        "headed by the agent badge: {r:?}"
    );
    finish(&mut p);
    let r = rows(&p);
    assert_eq!(
        r.last().unwrap(),
        "  \u{2713} fs:read src/foo.rs \u{00b7} 120 ms  use std;",
        "{r:?}"
    );
}

#[test]
fn the_pane_animates_while_a_call_is_pending_and_stops_when_it_returns() {
    let mut p = calling();
    assert!(p.tools_running(), "spinner and clock need frames");
    finish(&mut p);
    assert!(!p.tools_running());
    p.absorb_hive(&HiveEvent::ToolCall {
        agent: AgentId(7),
        label: "sys:run".into(),
        args: "{}".into(),
    });
    assert!(p.tools_running());
    p.fold_swarm();
    assert!(
        !p.tools_running(),
        "the run ending abandons what never returned"
    );
}

#[test]
fn the_reply_settling_collapses_the_block_to_a_summary_above_the_card() {
    let _g = crate::app::motion_test_guard();
    let _f = crate::glyphs::force(false);
    let mut p = calling();
    finish(&mut p);
    p.absorb_message(
        "coder \u{2192} user".into(),
        "done".into(),
        "500".into(),
        String::new(),
    );
    let r = rows(&p);
    let sum = r
        .iter()
        .position(|l| l.contains("tool call"))
        .expect("summary");
    assert_eq!(r[sum], "  \u{25b8} 1 tool call \u{00b7} 120 ms");
    assert!(r[sum + 1].contains("coder"), "right above the reply: {r:?}");
    assert!(
        !r.iter().any(|l| l.contains("\u{2713} fs:read")),
        "collapsed: {r:?}"
    );
}

#[test]
fn clicking_the_summary_opens_the_list_and_clicking_a_line_opens_its_text() {
    let _g = crate::app::motion_test_guard();
    let _f = crate::glyphs::force(false);
    let mut p = calling();
    finish(&mut p);
    p.absorb_message(
        "coder \u{2192} user".into(),
        "done".into(),
        "500".into(),
        String::new(),
    );
    let row = row_of(&p, "tool call");
    assert!(
        p.fold_target_at(COLS, ROWS, row),
        "the press arms on the summary"
    );
    assert!(p.toggle_fold_at(COLS, ROWS, row));
    let line = row_of(&p, "\u{2713} fs:read");
    assert!(
        p.fold_target_at(COLS, ROWS, line),
        "a done line with text is a target"
    );
    assert!(p.toggle_fold_at(COLS, ROWS, line));
    let r = rows(&p);
    assert!(
        r.iter().any(|l| l.contains("    fn main() {}")),
        "opened text: {r:?}"
    );
    assert!(p.toggle_fold_at(COLS, ROWS, line), "click again closes it");
    assert!(!rows(&p).iter().any(|l| l.contains("fn main")));
    assert!(p.toggle_fold_at(COLS, ROWS, row_of(&p, "tool call")));
    assert!(
        !rows(&p).iter().any(|l| l.contains("\u{2713} fs:read")),
        "collapsed again"
    );
    assert!(
        !p.toggle_fold_at(COLS, ROWS, row_of(&p, "plan: read")),
        "a plain card is not"
    );
}
