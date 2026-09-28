//! The router is a call of the turn it routes: a relay reply's turn total
//! counts it, exactly once, beside the relay's own calls.
use super::*;

const ROUTER: Spent = Spent {
    input: 100,
    output: 10,
    micros_usd: 1_000,
};

/// `(tokens, tok_in, tok_out, cost)` of every turn total the turn said.
fn totals(evs: &[PluginEvent]) -> Vec<(u64, u64, u64, u64)> {
    evs.iter()
        .filter_map(|e| match e {
            PluginEvent::Stats {
                agent,
                tokens,
                tok_in,
                tok_out,
                cost_microusd,
                ..
            } if agent.is_empty() => Some((*tokens, *tok_in, *tok_out, *cost_microusd)),
            _ => None,
        })
        .collect()
}

/// One routed reply on the mock relay, its router reporting `spent`.
fn reply_totals(spent: Spent) -> Vec<(u64, u64, u64, u64)> {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    let call = move |_: &str| Ok::<_, String>(("SHAPE: reply".to_string(), spent));
    let mut evs = Vec::new();
    let mut keep = |ev: PluginEvent| -> anyhow::Result<()> {
        evs.push(ev);
        Ok(())
    };
    let tick = crate::broker::tick::noop_tick_emit();
    route_counted(
        "hello there",
        Some(&call),
        &mut Session::new(),
        &tick,
        &mut keep,
    )
    .unwrap();
    assert!(
        pos_dial(&evs, "planner").is_some(),
        "the relay ran: {evs:?}"
    );
    totals(&evs)
}

#[test]
fn the_relay_turn_total_counts_the_router() {
    let alone = reply_totals(Spent::default());
    let [(tokens, tok_in, tok_out, cost)] = alone[..] else {
        panic!("the relay says one turn total: {alone:?}");
    };
    assert_eq!(
        reply_totals(ROUTER),
        vec![(tokens + 110, tok_in + 100, tok_out + 10, cost + 1_000)],
        "the relay's own calls, and the router's on top"
    );
}
