use super::*;

// ---- Pulse state ----

#[test]
fn record_hop_feeds_hops_and_history() {
    let mut p = Pulse::new();
    p.record_hop("planner", 2_000);
    p.record_hop("coder", 4_000);
    assert_eq!(
        p.hops(),
        &[("planner".to_string(), 2_000), ("coder".to_string(), 4_000)]
    );
    assert!(p.hist("planner").is_some());
    assert!(p.hist("reviewer").is_none());
}

#[test]
fn next_turn_resets_hops_but_keeps_history() {
    let mut p = Pulse::new();
    p.record_hop("planner", 2_000);
    p.end_turn();
    // The settled turn's hops stay put until the next turn's first hop…
    assert_eq!(p.hops().len(), 1);
    p.begin_hop();
    assert!(p.hops().is_empty(), "new turn starts a fresh hop list");
    assert!(
        p.hist("planner").is_some(),
        "sparkline history survives turns"
    );
}

#[test]
fn begin_hop_mid_turn_keeps_accumulating() {
    let mut p = Pulse::new();
    p.begin_hop();
    p.record_hop("planner", 1_000);
    p.begin_hop(); // next hop of the same turn
    p.record_hop("coder", 1_000);
    assert_eq!(p.hops().len(), 2);
}
