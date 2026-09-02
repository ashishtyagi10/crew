//! The two rows under the `/tools` heading that say what KIND of history this
//! is before you read a line of it: how many calls of each tier are in view,
//! and how many did not simply run.
//!
//! The listing tails a thousand rows and filters on a word, and never said
//! how many of what it was showing — "what was denied", the question an
//! audit surface gets asked first, meant scanning for `✗`.
use crew_plugin::ledger::Record;

/// The tiers, in the order a person ranks them.
const TIERS: [&str; 3] = ["read", "reversible", "irreversible"];

/// One row per line: the tiers (always, for what is in view), then the
/// unusual endings — only when there are any, since `0 denied` on every
/// listing is the kind of reassurance nobody reads twice.
pub(crate) fn tally(hits: &[&Record]) -> Vec<String> {
    let count = |f: &dyn Fn(&Record) -> bool| hits.iter().filter(|r| f(r)).count();
    let mut tiers: Vec<String> = TIERS
        .iter()
        .map(|t| (count(&|r| r.tier == *t), t))
        .filter(|(n, _)| *n > 0)
        .map(|(n, t)| format!("{n} {t}"))
        .collect();
    let other = count(&|r| !TIERS.contains(&r.tier.as_str()));
    if other > 0 {
        tiers.push(format!("{other} other"));
    }
    let denied = count(&|r| r.decision == "deny" || r.outcome == "denied");
    let failed = count(&|r| matches!(r.outcome.as_str(), "failed" | "timed_out"));
    let pending = count(&|r| r.outcome.is_empty() && r.decision != "deny");
    let unusual: Vec<String> = [(denied, "denied"), (failed, "failed"), (pending, "pending")]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, w)| format!("{n} {w}"))
        .collect();
    let mut rows = vec![tiers.join(" \u{b7} ")];
    if !unusual.is_empty() {
        rows.push(unusual.join(" \u{b7} "));
    }
    rows
}

#[cfg(test)]
#[path = "toolstally_tests.rs"]
mod tests;
