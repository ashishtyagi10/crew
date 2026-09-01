use super::*;
// The card-identity tests still build whole cards to look at them, so they
// reach the layout half and its `msg` fixture.
use crate::chatmsgs::tests::{card_row, card_top, msg};
use crate::chatmsgs::{card_line_count, message_cells, View};

#[test]
fn card_has_header_then_indented_body() {
    let cells = message_cells(&[&msg("planner", "hello")], 40, 10, 0, 0, View::default());
    assert_eq!(card_row(&cells, 0), format!("{GUTTER}planner"));
    assert_eq!(card_row(&cells, 1), " hello");
}

#[test]
fn cards_are_separated_by_a_blank_line() {
    let m = [msg("planner", "a"), msg("coder", "b")];
    let refs: Vec<&Message> = m.iter().collect();
    let cells = message_cells(&refs, 40, 10, 0, 0, View::default());
    assert_eq!(card_row(&cells, 2), ""); // spacer
    assert_eq!(card_row(&cells, 3), format!("{GUTTER}coder"));
}

#[test]
fn header_tail_keeps_relative_time_but_drops_latency() {
    // The muted card-header tail carries "when" (relative time), never the
    // per-card reply latency — reductionist chrome, one signal per question.
    let m = Message {
        sender: "coder".into(),
        text: "done".into(),
        ts: "999700000".into(),
        meta: "4.2s".into(),
        usage: None,
        expanded: false,
    };
    let chars: String = header_line(&m, 1_000_000_000, None)
        .iter()
        .map(|c| c.c)
        .collect();
    assert!(chars.contains("5m ago"), "relative time shown: {chars}");
    assert!(!chars.contains("4.2s"), "latency must be gone: {chars}");
}

#[test]
fn handoff_sender_colours_each_name_separately() {
    let _g = crate::app::theme_test_guard();
    let cells = message_cells(
        &[&msg("planner \u{2192} coder", "x")],
        40,
        10,
        0,
        0,
        View::default(),
    );
    assert_eq!(
        card_row(&cells, 0),
        format!("{GUTTER}planner \u{2192} coder")
    );
    let muted = crew_theme::theme().text_muted;
    let hdr = card_top(&cells);
    let cell_at = |col: u16| cells.iter().find(|c| c.row == hdr && c.col == col).unwrap();
    assert_ne!(cell_at(1).fg, muted, "planner keeps its agent colour");
    assert_ne!(cell_at(11).fg, muted, "coder keeps its agent colour");
}

#[test]
fn system_sender_is_muted_and_agents_are_not() {
    let _g = crate::app::theme_test_guard();
    assert_eq!(sender_color("crew"), crew_theme::theme().text_muted);
    assert_ne!(sender_color("planner"), crew_theme::theme().text_muted);
}

#[test]
fn agent_message_keeps_the_solid_gutter() {
    let cells = message_cells(
        &[&msg("planner \u{2192} user", "hello")],
        40,
        10,
        0,
        0,
        View::default(),
    );
    assert_eq!(
        card_row(&cells, 0),
        format!("{GUTTER}planner \u{2192} user")
    );
}

#[test]
fn count_matches_rendered_lines_and_scroll_shows_older() {
    let m = [msg("a", "one"), msg("b", "two")];
    let refs: Vec<&Message> = m.iter().collect();
    // 2 cards × (header + body) + 1 spacer = 5 lines.
    assert_eq!(card_line_count(&refs, 40, View::default()), 5);
    // A 2-row window scrolled 3 up from the bottom shows the first card.
    let cells = message_cells(&refs, 40, 2, 0, 3, View::default());
    assert_eq!(card_row(&cells, 0), format!("{GUTTER}a"));
}

#[test]
fn header_line_shows_a_dim_chip_for_task_tagged_messages() {
    let _g = crate::app::theme_test_guard();
    let m = Message {
        sender: "planner \u{2192} user".into(),
        text: "done".into(),
        ts: String::new(),
        meta: "task:2 \u{00b7} 0.0s".into(),
        usage: None,
        expanded: false,
    };
    let line = header_line(&m, 0, None);
    let muted = crew_theme::theme().text_muted;
    let hash = line.iter().find(|c| c.c == '#').expect("chip # present");
    assert_eq!(hash.fg, muted, "chip # is muted");
    let id = line.iter().find(|c| c.c == '2').expect("chip id present");
    assert_eq!(id.fg, muted, "chip id is muted");
    let chars: String = line.iter().map(|c| c.c).collect();
    assert!(
        !chars.contains("0.0s"),
        "per-card latency must be gone: {chars}"
    );
    assert!(
        !chars.contains("task"),
        "tag must not leak into the header: {chars}"
    );
}

#[test]
fn header_line_has_no_chip_for_untagged_messages() {
    let mut m = msg("coder", "done");
    m.meta = "4.2s".into();
    let line = header_line(&m, 0, None);
    assert!(
        !line.iter().any(|c| c.c == '#'),
        "no task tag means no chip"
    );
}

#[test]
fn a_tool_card_takes_the_quiet_gutter_and_muted_ink() {
    let _g = crate::app::theme_test_guard();
    let tool = msg("api-consumer", "[tool] sys:run  curl -s wttr.in/Oslo");
    let reply = msg("api-consumer", "Oslo is 4C and cloudy.");
    let muted = crew_theme::theme().text_muted;

    let t = header_line(&tool, 0, None);
    let r = header_line(&reply, 0, None);

    // Same sender, two voices: dotted vs solid gutter.
    assert_eq!(t[0].c, '\u{2506}', "tool gutter");
    assert_eq!(r[0].c, GUTTER, "reply gutter");
    // …and the tool card's name is muted, so a run of four calls does not read
    // as four replies in the agent's own colour.
    assert_eq!(t[0].fg, muted);
    assert_ne!(
        r[0].fg, muted,
        "an agent's actual reply must keep its roster colour"
    );
}

#[test]
fn the_tool_predicate_does_not_capture_an_agent_quoting_the_marker() {
    // Mid-sentence, not a tool line.
    assert!(!is_tool_card(&msg("coder", "I would write [tool] here")));
    // The system voice has its own treatment already; it must not be
    // reclassified, or the splash and turn summaries change fold depth.
    assert!(!is_tool_card(&msg("agent smith", "[tool] sys:run x")));
    assert!(is_tool_card(&msg("coder", "[tool] sys:run x")));
}
