use super::tally;
use crew_plugin::ledger::Record;

fn rec(tier: &str, decision: &str, outcome: &str) -> Record {
    Record {
        ts_ms: 0,
        tool: "t:t".into(),
        tier: tier.into(),
        requester: "pane".into(),
        decision: decision.into(),
        outcome: outcome.into(),
        note: String::new(),
    }
}

#[test]
fn the_tiers_are_counted_in_rank_order_and_zeroes_are_left_out() {
    let recs = [
        rec("irreversible", "allow", "ran"),
        rec("read", "allow", "ran"),
        rec("read", "allow", "ran"),
    ];
    let hits: Vec<&Record> = recs.iter().collect();
    assert_eq!(tally(&hits), vec!["2 read \u{b7} 1 irreversible"]);
}

#[test]
fn the_unusual_endings_get_a_second_row_only_when_there_are_any() {
    let recs = [
        rec("read", "allow", "ran"),
        rec("irreversible", "deny", ""),
        rec("reversible", "allow", "failed"),
        rec("read", "allow", "timed_out"),
        rec("irreversible", "ask", ""),
        rec("weird", "allow", "ran"),
    ];
    let hits: Vec<&Record> = recs.iter().collect();
    assert_eq!(
        tally(&hits),
        vec![
            "2 read \u{b7} 1 reversible \u{b7} 2 irreversible \u{b7} 1 other",
            "1 denied \u{b7} 2 failed \u{b7} 1 pending",
        ]
    );
    // A denial is not "pending": its outcome is empty by design.
    let only_deny = [rec("read", "deny", "")];
    let hits: Vec<&Record> = only_deny.iter().collect();
    assert_eq!(tally(&hits), vec!["1 read", "1 denied"]);
}
