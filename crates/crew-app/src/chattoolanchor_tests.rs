//! Where a block of loads alone ends up: above the next reply, whoever
//! sends it — and never stolen from an agent that has a block of its own.
use super::tests::{call, loaded};
use super::*;
use crate::chatmsgs::card_lines;
use crate::chatmsgs::tests::msg;
use crate::chatmsgs::View;
use crate::chattoolview::{above_of, orphan_ids};

fn text(l: &crate::chatbody::CardLine) -> String {
    l.iter().map(|c| c.c).collect()
}

#[test]
fn a_block_of_loads_alone_anchors_above_the_next_reply_whoever_sends_it() {
    let _g = crate::app::motion_test_guard();
    let mut t = ToolLines::default();
    t.absorb(&loaded(LEAD, "skill", "review", "applied \u{b7} strict"), 0);
    // A block with a real call is NOT handed over: it belongs to its agent.
    t.absorb(&call(4), 0);
    t.settle("coder", "77");
    assert_eq!(t.blocks[0].agent, "coder", "the load block became coder's");
    assert_eq!(t.blocks[0].anchor.as_deref(), Some("77"));
    assert!(t.blocks[0].settled && !t.blocks[0].expanded);
    assert!(!t.blocks[1].settled, "agent 4's live block is untouched");
    let mut reply = msg("coder \u{2192} user", "done");
    reply.ts = "77".into();
    let view = View {
        tools: &t.blocks,
        ..View::default()
    };
    assert_eq!(above_of(view, &reply, false), Some(0));
    assert_eq!(
        orphan_ids(view, &[&reply]),
        [1],
        "only the live block stands alone"
    );
    let rows: Vec<String> = card_lines(&[&reply], 60, 0, view)
        .iter()
        .map(text)
        .collect();
    assert_eq!(
        rows[0], "  \u{25b8} 1 skill",
        "the summary above the reply: {rows:?}"
    );
}

#[test]
fn a_reply_from_an_agent_with_its_own_block_leaves_the_load_block_waiting() {
    let mut t = ToolLines::default();
    t.absorb(&loaded(LEAD, "skill", "review", "applied"), 0);
    t.absorb(&call(4), 0);
    t.bind("coder");
    t.settle("coder", "77");
    assert_eq!(
        t.blocks[1].anchor.as_deref(),
        Some("77"),
        "coder's own block"
    );
    assert!(
        !t.blocks[0].settled,
        "the load block waits for the next reply"
    );
}
