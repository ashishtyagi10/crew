use super::*;
use crate::chattoolline::render;
use crew_hive::{AgentId, HiveEvent};

pub(super) fn loaded(agent: &str, kind: &str, name: &str, detail: &str) -> HiveEvent {
    HiveEvent::Loaded {
        agent: agent.into(),
        kind: kind.into(),
        name: name.into(),
        detail: detail.into(),
    }
}

pub(super) fn call(agent: u64) -> HiveEvent {
    HiveEvent::ToolCall {
        agent: AgentId(agent),
        label: "fs:read".into(),
        args: r#"{"path":"a"}"#.into(),
    }
}

fn text(l: &crate::chatbody::CardLine) -> String {
    l.iter().map(|c| c.c).collect()
}

#[test]
fn a_load_is_a_done_line_with_its_kinds_mark_and_no_clock() {
    let _g = crate::app::motion_test_guard();
    let mut t = ToolLines::default();
    t.absorb(
        &loaded(
            "coder",
            "skill",
            "rust-testing",
            "applied \u{b7} tests first",
        ),
        5_000,
    );
    assert_eq!(t.pending(), 0, "nothing ran, nothing is in flight");
    let b = &t.blocks[0];
    assert_eq!(b.agent, "coder");
    let l = &b.lines[0];
    assert_eq!(
        (l.kind, l.label.as_str()),
        (LineKind::Skill, "skill rust-testing")
    );
    assert_eq!(l.done.as_ref().unwrap().text, "applied \u{b7} tests first");
    assert!(l.has_text(), "a click opens the detail");
    assert_eq!(
        text(&render(l, 99_000, 60, false)),
        "  \u{2726} skill rust-testing \u{b7} applied \u{b7} tests first",
        "no ms, no seconds, no result preview"
    );
    let nerd = render(l, 0, 60, true);
    assert_eq!(nerd[2].c, '\u{f0e7}', "the bolt on the icon set");
    t.absorb(
        &loaded("coder", "mcp", "github", "connected \u{b7} 2 tools: a, b"),
        0,
    );
    t.absorb(
        &loaded("coder", "lsp", "rust-analyzer", "rust \u{b7} crew"),
        0,
    );
    let kinds: Vec<LineKind> = t.blocks[0].lines.iter().map(|l| l.kind).collect();
    assert_eq!(kinds, [LineKind::Skill, LineKind::Mcp, LineKind::Lsp]);
    assert_eq!(
        text(&render(&t.blocks[0].lines[2], 0, 60, false)),
        "  \u{3bb} lsp rust-analyzer \u{b7} rust \u{b7} crew"
    );
    assert_eq!(
        text(&render(&t.blocks[0].lines[1], 0, 60, false))
            .chars()
            .nth(2),
        Some('\u{21c4}')
    );
}

#[test]
fn an_unknown_kind_is_dropped_not_mislabelled() {
    let mut t = ToolLines::default();
    t.absorb(&loaded("coder", "hologram", "x", "y"), 0);
    assert!(t.blocks.is_empty());
}

#[test]
fn a_load_naming_nobody_joins_the_active_agents_block_else_the_lead() {
    let mut t = ToolLines::default();
    t.absorb(
        &loaded("", "mcp", "github", "connected \u{b7} 1 tool: x"),
        0,
    );
    assert_eq!(t.blocks[0].agent, LEAD, "no agent active yet: the lead");
    t.absorb(&call(3), 0);
    t.absorb(&loaded("", "lsp", "rust-analyzer", "rust \u{b7} crew"), 0);
    assert_eq!(t.blocks.len(), 2);
    assert_eq!(t.blocks[1].agent_id, 3);
    assert_eq!(
        t.blocks[1].lines.iter().map(|l| l.kind).collect::<Vec<_>>(),
        [LineKind::Tool, LineKind::Lsp],
        "under the call that forced the start"
    );
}

#[test]
fn the_summary_counts_calls_then_loads_by_kind_and_times_only_the_calls() {
    let mut t = ToolLines::default();
    t.absorb(&call(1), 0);
    t.absorb(&call(1), 0);
    t.absorb(
        &HiveEvent::ToolResult {
            agent: AgentId(1),
            label: "fs:read".into(),
            ok: true,
            text: String::new(),
            ms: 2_000,
        },
        0,
    );
    t.absorb(
        &HiveEvent::ToolResult {
            agent: AgentId(1),
            label: "fs:read".into(),
            ok: false,
            text: String::new(),
            ms: 100,
        },
        0,
    );
    t.absorb(&loaded("", "skill", "review", "applied"), 0);
    t.absorb(&loaded("", "mcp", "gh", "connected"), 0);
    let b = &t.blocks[0];
    assert_eq!(b.tally(), (2, 1, 2_100), "loads are not calls");
    assert_eq!(
        summary_text(b),
        "2 tool calls \u{b7} 1 skill \u{b7} 1 mcp server \u{b7} 2.1 s"
    );
    let mut only = ToolLines::default();
    only.absorb(&loaded("coder", "skill", "a", ""), 0);
    only.absorb(&loaded("coder", "skill", "b", ""), 0);
    assert_eq!(summary_text(&only.blocks[0]), "2 skills", "nothing to time");
    assert_eq!(
        summary_text(&ToolLines::default().open_block(9).clone()),
        "0 tool calls \u{b7} 0 ms"
    );
}
