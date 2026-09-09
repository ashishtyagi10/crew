use crate::chat::ChatPane;
use crate::motion::{set_level, MotionLevel};
use crew_plugin::Plugin;

fn thinking(agent: &str) -> ChatPane {
    // An idle child stands in for the broker; only pane state is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.absorb_activity(agent.into(), "thinking", "user".into());
    p
}

#[test]
fn a_token_burst_lights_the_agent_name_then_it_settles_back() {
    let _g = crate::app::motion_test_guard();
    set_level(MotionLevel::Full);
    let mut p = thinking("coder");
    let roster = crate::chatroster::agent_color("coder");
    let (label, _, before) = p.header_active(1_000).expect("thinking");
    assert_eq!(label, "coder");
    assert_eq!(before, roster, "no burst yet: the roster colour");
    // The relay stamps `coder → user`; the pulse keys by the agent.
    p.note_tokens("coder \u{2192} user", 1_000);
    let (_, _, lit) = p.header_active(1_010).unwrap();
    assert_ne!(lit, roster, "lit toward the ink right after the burst");
    let (_, _, later) = p.header_active(1_000 + crate::shimmer::PULSE_MS).unwrap();
    assert_eq!(
        later, roster,
        "and back on the roster colour after PULSE_MS"
    );
}

#[test]
fn off_never_lights_the_name() {
    let _g = crate::app::motion_test_guard();
    set_level(MotionLevel::Off);
    let mut p = thinking("coder");
    p.note_tokens("coder", 1_000);
    let (_, _, c) = p.header_active(1_010).unwrap();
    assert_eq!(c, crate::chatroster::agent_color("coder"));
}

#[test]
fn nobody_thinking_is_no_label() {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let p = ChatPane::new(plugin, "crew".into());
    assert!(p.header_active(1_000).is_none());
}
