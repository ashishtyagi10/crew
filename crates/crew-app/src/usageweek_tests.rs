//! The week's header line, empty and not.
use super::week_line;
use crate::usageledger::{Buckets, DAYS, HOURS};

fn week(cost: u64, tin: u64, tout: u64) -> Buckets {
    Buckets {
        hourly: vec![0; DAYS * HOURS],
        daily_cost: vec![0; DAYS],
        tok_in: tin,
        tok_out: tout,
        cost_microusd: cost,
    }
}

#[test]
fn an_empty_week_says_so_and_a_used_one_counts() {
    assert_eq!(
        week_line(&week(0, 0, 0), " · "),
        "nothing used in the last 7 days"
    );
    assert_eq!(
        week_line(&week(1_980_000, 1_840_000, 410_000), " · "),
        "$1.98 · 1.8M in · 410k out · 7 days"
    );
}
