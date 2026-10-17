use crew_hive::{AgentId, HiveEvent};
use crew_plugin::PluginEvent;

use super::*;

fn call(label: &str) -> PluginEvent {
    PluginEvent::Hive {
        event: HiveEvent::ToolCall {
            agent: AgentId::minted("claude"),
            label: label.into(),
            args: "{}".into(),
        },
    }
}

fn card(text: &str) -> PluginEvent {
    PluginEvent::Message {
        channel: "crew".into(),
        sender: "engineer".into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
    }
}

#[test]
fn claude_codes_tools_and_crews_land_in_the_same_phases() {
    for (label, want) in [
        ("Read", Phase::Tracing),
        ("Grep", Phase::Tracing),
        ("sys:read_file", Phase::Tracing),
        ("lsp:references", Phase::Tracing),
        ("Edit", Phase::Rewriting),
        ("sys:write_file", Phase::Rewriting),
        ("sys:edit", Phase::Rewriting),
        ("Bash", Phase::Compiling),
        ("sys:run", Phase::Compiling),
        ("sys:git", Phase::Decrypting),
        ("WebFetch", Phase::Reaching),
        ("sys:search", Phase::Reaching),
        ("Task", Phase::Replicating),
        ("mcp__github__create_issue", Phase::Other),
    ] {
        assert_eq!(of_tool(label), want, "{label}");
    }
}

#[test]
fn every_phase_has_its_own_word_and_none_is_a_tool_name() {
    use Phase::*;
    let all = [
        Routing,
        Thinking,
        Tracing,
        Rewriting,
        Compiling,
        Decrypting,
        Reaching,
        Replicating,
        Answering,
        Other,
    ];
    let words: std::collections::BTreeSet<&str> = all.iter().map(|p| p.word()).collect();
    assert_eq!(words.len(), all.len());
    for w in words {
        assert!(!["read", "edit", "bash", "grep", "run"].contains(&w), "{w}");
    }
}

#[test]
fn the_api_engines_tool_cards_say_the_phase_too() {
    assert_eq!(
        of_event(&card("[tool] sys:git  log")),
        Some(Phase::Decrypting)
    );
    let result = "[tool] sys:git \u{2713} 0.0s\nfe5b98c merge";
    assert_eq!(of_event(&card(result)), Some(Phase::Thinking));
    assert_eq!(
        of_event(&card("the answer")),
        None,
        "a reply is not a phase"
    );
}

#[test]
fn a_result_or_a_thought_is_thinking_and_the_reply_is_the_answer() {
    let result = PluginEvent::Hive {
        event: HiveEvent::ToolResult {
            agent: AgentId::minted("claude"),
            label: "Read".into(),
            ok: true,
            text: String::new(),
            ms: 4,
        },
    };
    assert_eq!(of_event(&result), Some(Phase::Thinking));
    let delta = |sub| PluginEvent::Delta {
        agent: "smith".into(),
        text: "so".into(),
        sub,
    };
    assert_eq!(of_event(&delta(false)), Some(Phase::Answering));
    assert_eq!(
        of_event(&delta(true)),
        None,
        "a sub-agent's stream is not smith's"
    );
}

#[test]
fn a_turn_starts_rerouting_and_a_new_phase_shows_at_once_after_the_dwell() {
    let mut c = PhaseClock::start(1_000);
    assert_eq!(c.at(1_000), (Phase::Routing, 1_000));
    c.hear(&call("Read"), 5_000);
    assert_eq!(
        c.at(5_000),
        (Phase::Tracing, 5_000),
        "routing held long enough"
    );
}

#[test]
fn a_burst_of_calls_does_not_restart_the_word_inside_the_dwell() {
    let mut c = PhaseClock::start(0);
    c.hear(&call("Read"), 2_000);
    assert_eq!(c.at(2_000).0, Phase::Tracing);
    c.hear(&call("Edit"), 2_300);
    assert_eq!(
        c.at(2_400),
        (Phase::Tracing, 2_000),
        "tracing holds its 1.5 s"
    );
    assert_eq!(
        c.at(3_500),
        (Phase::Rewriting, 3_500),
        "then rewriting takes over"
    );
    c.hear(&call("Grep"), 3_600);
    c.hear(&call("Edit"), 3_700);
    assert_eq!(
        c.at(9_000).0,
        Phase::Rewriting,
        "a flicker back cancels the wait"
    );
}

#[test]
fn the_same_phase_again_keeps_its_time() {
    let mut c = PhaseClock::start(0);
    c.hear(&call("Read"), 2_000);
    c.hear(&call("Grep"), 2_600);
    assert_eq!(
        c.at(2_700),
        (Phase::Tracing, 2_000),
        "a second read is the same word"
    );
}

#[test]
fn the_pane_hears_the_phase_on_its_own_event_path() {
    let mut p = crate::chat::tests::pane();
    p.heard(&call("Bash"));
    let now = crate::anim::now_ms();
    assert_eq!(p.watch.phase.word_at(now).0, "compiling");
}
