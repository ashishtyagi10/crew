//! Round constants are BACKSTOPS, not drivers: the model can end a loop
//! early by putting `@done` on the first line of a round's answer (the same
//! token `route.rs`'s relay protocol already uses), and a run that never
//! declares done still stops at the numeric ceiling.
use super::*;
use crate::broker::testenv;
use crate::PluginEvent;

fn texts(evs: &[PluginEvent]) -> Vec<String> {
    evs.iter()
        .filter_map(|e| match e {
            PluginEvent::Message { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Drive [`rounds`] with a stubbed turn — the seam that lets a test vary the
/// answer per round, which a static mock reply cannot.
fn run_rounds(n: u32, turn: &mut dyn FnMut(u32, &str) -> Option<String>) -> (Vec<String>, u32) {
    let session = Session::new();
    let mut evs = Vec::new();
    let mut calls = 0u32;
    let mut turn_adapter = |round: u32,
                            body: &str,
                            _emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>|
     -> anyhow::Result<Option<String>> {
        calls += 1;
        Ok(turn(round, body))
    };
    rounds(
        &session,
        n,
        "coder",
        "polish it",
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
        &mut turn_adapter,
    )
    .unwrap();
    (texts(&evs), calls)
}

#[test]
fn a_turn_declaring_done_on_round_two_ends_a_five_round_loop_early() {
    let mut bodies = Vec::new();
    let (ts, calls) = run_rounds(5, &mut |round, body| {
        bodies.push(body.to_string());
        Some(if round == 2 {
            "@done\nfinal version".to_string()
        } else {
            "a better draft".to_string()
        })
    });
    assert_eq!(calls, 2, "rounds 3-5 never ran: {ts:?}");
    assert!(
        ts.iter().any(|t| t.contains("done early after 2 round(s)")),
        "{ts:?}"
    );
    assert_eq!(
        ts.iter().filter(|t| t.starts_with("loop round")).count(),
        2,
        "{ts:?}"
    );
    // The agent is TOLD the idiom once there is a result to keep: a budget —
    // or an exit — the model cannot see is one it plans straight past.
    assert!(
        bodies[1].contains("@done"),
        "round 2's body never taught the early exit: {}",
        bodies[1]
    );
    assert!(
        !bodies[0].contains("@done"),
        "round 1 has nothing to keep yet: {}",
        bodies[0]
    );
}

#[test]
fn a_never_done_turn_still_stops_at_the_numeric_ceiling() {
    // The ceiling itself, pinned numerically: it must survive as a backstop.
    assert_eq!(MAX_ROUNDS, 10);
    let (ts, calls) = run_rounds(MAX_ROUNDS, &mut |_, _| Some("keep going".into()));
    assert_eq!(calls, 10, "{ts:?}");
    assert!(
        ts.iter().any(|t| t.contains("10 round(s) complete")),
        "{ts:?}"
    );
    assert!(!ts.iter().any(|t| t.contains("done early")), "{ts:?}");
}

/// The same early exit through the REAL relay path: a mocked agent whose
/// answer leads with `@done` ends a five-round loop on round one.
#[test]
fn a_mocked_agent_declaring_done_ends_the_loop_from_the_relay_path() {
    let _g = testenv::mock_with_specialists("@done\nnothing left to improve\n@done", testenv::TRIO);
    let mut session = Session::new();
    let mut evs = Vec::new();
    loop_cmd(
        &mut session,
        "5 polish the haiku",
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            evs.push(ev);
            Ok(())
        },
    )
    .unwrap();
    let ts = texts(&evs);
    assert!(
        ts.iter().any(|t| t.contains("done early after 1 round")),
        "{ts:?}"
    );
    assert_eq!(
        ts.iter().filter(|t| t.starts_with("loop round")).count(),
        1,
        "{ts:?}"
    );
}

/// `/goal`'s ceiling, pinned numerically — its early exit (the judge's MET
/// verdict) is covered by `constructs_tests::goal_met_on_round_one`.
#[test]
fn the_goal_ceiling_is_five_rounds() {
    assert_eq!(crate::broker::constructs::GOAL_ROUNDS, 5);
}
