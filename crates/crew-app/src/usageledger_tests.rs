use super::*;

const H: u64 = 3_600_000;

fn ledger(entries: &[(u64, u64)]) -> Ledger {
    // (ts_ms, tokens) — split evenly across in/out; cost 0.
    let entries = entries
        .iter()
        .map(|&(ts_ms, tok)| Entry {
            ts_ms,
            tok_in: tok / 2,
            tok_out: tok - tok / 2,
            cost_microusd: 0,
        })
        .collect();
    Ledger {
        entries,
        budget_5h: 1_000,
        budget_7d: 10_000,
    }
}

#[test]
fn no_usage_means_no_open_window() {
    assert!(ledger(&[]).window(100 * H, 5 * H, 1_000).is_none());
}

#[test]
fn block_opens_at_first_use_and_counts_spend_and_time_left() {
    // First use at t=10h opens a 5h block [10h, 15h).
    let l = ledger(&[(10 * H, 300), (12 * H, 200)]);
    let w = l.window(13 * H, 5 * H, 1_000).unwrap();
    assert_eq!(w.left_ms, 2 * H);
    assert_eq!(w.spent, 500);
    assert_eq!(w.budget, 1_000);
}

#[test]
fn expired_block_closes_and_next_use_opens_a_new_one() {
    // Block [10h,15h) expired; nothing since → no open window at t=16h.
    let l = ledger(&[(10 * H, 300)]);
    assert!(l.window(16 * H, 5 * H, 1_000).is_none());
    // A later entry at t=17h opens [17h,22h) containing only its own spend.
    let l = ledger(&[(10 * H, 300), (17 * H, 50)]);
    let w = l.window(18 * H, 5 * H, 1_000).unwrap();
    assert_eq!(w.left_ms, 4 * H);
    assert_eq!(w.spent, 50);
}

#[test]
fn chained_blocks_reset_on_boundaries_not_on_gaps() {
    // Use at 10h opens [10h,15h); use at 16h (after expiry) opens [16h,21h).
    let l = ledger(&[(10 * H, 300), (16 * H, 100), (20 * H, 100)]);
    let w = l.window(20 * H, 5 * H, 1_000).unwrap();
    assert_eq!(w.spent, 200);
    assert_eq!(w.left_ms, H);
}

#[test]
fn prune_drops_entries_older_than_seven_days() {
    let mut l = ledger(&[(0, 100), (8 * 24 * H, 100)]);
    l.prune(8 * 24 * H + H);
    assert_eq!(l.entries.len(), 1);
}

#[test]
fn zero_usage_turns_never_open_a_rolling_window() {
    // Zero-usage (0,0) must not be recorded; non-zero usage opens windows.
    assert!(!super::records_usage(0, 0));
    assert!(super::records_usage(1, 0));
    assert!(super::records_usage(0, 1));
    assert!(super::records_usage(1, 1));
}
