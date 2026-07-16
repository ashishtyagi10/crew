//! The folded swarm record: when a run ends, `chatswarm` retires the live
//! status line into a transcript message rendered here — a markdown task list
//! plus a Σ line totalling the run's tokens, spend and wall-clock duration.

use crew_hive::TaskState;

use crate::chatswarm::SwarmStatus;
use crate::chattime::fmt_elapsed;

impl SwarmStatus {
    /// The block as a markdown list — the transcript record on fold.
    ///
    /// `run_ms` is the run's wall-clock duration, supplied by the caller
    /// rather than read from `run_started` here: `Instant` can't be mocked, so
    /// reading the clock inside would make every test's Σ line read "0.0s".
    /// `None` omits the duration.
    pub(crate) fn record_text(&self, run_ms: Option<u64>) -> String {
        let mut out = self
            .tasks
            .iter()
            .map(|t| {
                let glyph = glyph(&t.state);
                let mut line = if t.tokens > 0 {
                    format!("- {glyph} {} — {} tok", t.title, fmt_tok(t.tokens))
                } else {
                    format!("- {glyph} {}", t.title)
                };
                if t.cost_micros > 0 {
                    line.push_str(" \u{00b7} ");
                    line.push_str(&fmt_cost(t.cost_micros));
                }
                if let Some(ms) = t.elapsed_ms {
                    line.push_str(" \u{00b7} ");
                    line.push_str(&fmt_elapsed(ms));
                }
                line
            })
            .collect::<Vec<_>>()
            .join("\n");
        // Run totals — the only place the whole run's spend surfaces in chat
        // (the broker's aggregate Stats carries tokens but not cost).
        //
        // Gated on the run having consumed something: a run cancelled before
        // it started would otherwise summarise itself as "Σ 0 tok · 0.0s".
        // Cost is absent on keyless/stub runs, which still report tokens — so
        // those get a Σ line, just without the `$` part.
        let cost: u64 = self.tasks.iter().map(|t| t.cost_micros).sum();
        let tok: u64 = self.tasks.iter().map(|t| t.tokens).sum();
        if let Some(ms) = run_ms.filter(|_| tok > 0 || cost > 0) {
            out.push_str(&format!("\n\n\u{03a3} {} tok", fmt_tok(tok)));
            if cost > 0 {
                out.push_str(&format!(" \u{00b7} {}", fmt_cost(cost)));
            }
            out.push_str(&format!(" \u{00b7} {}", fmt_elapsed(ms)));
        }
        out
    }
}

/// Compact token count (`"12.4k"` past 1000) — shared by the live block
/// (`chatswarmview`) and the folded transcript record so the two never show
/// different numbers for the same run.
pub(crate) fn fmt_tok(n: u64) -> String {
    if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        n.to_string()
    }
}

/// Micro-USD as dollars — 2 decimals once the value *displays* as a cent
/// (from 9,950 micros, where `{:.4}` would round to the inconsistent
/// `$0.0100`), 4 below it so sub-cent task costs don't collapse to `$0.00`,
/// and `<$0.0001` under 50 micros (where even 4 decimals round to zero).
/// Shared by the live block and the folded record.
pub(crate) fn fmt_cost(micros: u64) -> String {
    let usd = micros as f64 / 1_000_000.0;
    if micros >= 9_950 {
        format!("${usd:.2}")
    } else if micros >= 50 {
        format!("${usd:.4}")
    } else {
        "<$0.0001".into()
    }
}

/// The state glyph shared by the live block and the folded record.
pub(crate) fn glyph(state: &TaskState) -> char {
    match state {
        TaskState::Pending | TaskState::Ready => '·',
        TaskState::Running => '⠿', // live view animates; record shows a static mark
        TaskState::Done => '✓',
        TaskState::Failed => '✗',
        TaskState::Cancelled => '⊘',
    }
}
