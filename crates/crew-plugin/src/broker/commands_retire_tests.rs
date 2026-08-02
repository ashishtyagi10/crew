//! Retired commands: `/fan` and `/loop` left the construct surface (their
//! capability now answers plain language through the intent router), and the
//! router must say so helpfully — instantly, with the replacement phrasing,
//! never a silent reinterpretation or a generic did-you-mean.
use super::*;

fn run(text: &str) -> Vec<PluginEvent> {
    let mut session = Session::new();
    let mut out = Vec::new();
    handle(
        &mut session,
        text,
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            out.push(ev);
            Ok(())
        },
    )
    .unwrap();
    out
}

fn text_of(ev: &PluginEvent) -> &str {
    match ev {
        PluginEvent::Message { text, .. } => text,
        _ => "",
    }
}

#[test]
fn retired_fan_and_loop_teach_the_plain_language_ask() {
    for cmd in ["/fan build it", "/loop 3 improve it"] {
        let evs = run(cmd);
        let t = text_of(&evs[0]);
        assert!(t.contains("retired"), "{cmd}: {t}");
        // The dedicated hint, not the generic unknown-construct arm: the user
        // must be taught the phrasing, not offered a different slash command.
        assert!(!t.contains("unknown construct"), "{cmd}: {t}");
        assert!(!t.contains("did you mean"), "{cmd}: {t}");
    }
}

#[test]
fn retired_commands_answer_inline() {
    // The hint dials no agent, so a retired command must never occupy a
    // worker slot.
    for cmd in ["/fan build it", "/loop 3 x"] {
        assert!(is_quick(cmd), "{cmd}");
    }
}

#[test]
fn help_teaches_plain_language_instead_of_fan_and_loop() {
    let evs = run("/help");
    let t = text_of(&evs[0]);
    assert!(!t.contains("/fan"), "{t}");
    assert!(!t.contains("/loop"), "{t}");
    // The replacement: at least one example phrasing that the intent router
    // turns into a fan-out or a loop.
    assert!(t.contains("have every agent take a crack at"), "{t}");
    assert!(t.contains("keep refining"), "{t}");
}
