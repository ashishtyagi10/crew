//! The folded swarm record: when a run ends, `chatswarm` retires the live
//! block into a transcript message rendered here — a markdown task list plus,
//! for concurrent runs, a `chattimeline` Gantt block showing each task's
//! span within the run.

use crew_hive::TaskState;

use crate::chatswarm::SwarmStatus;
use crate::chattime::fmt_elapsed;
use crate::chattimeline::timeline_block;

impl SwarmStatus {
    /// The block as a markdown list (+ timeline) — the transcript record on
    /// fold.
    pub(crate) fn record_text(&self) -> String {
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
        let cost: u64 = self.tasks.iter().map(|t| t.cost_micros).sum();
        if cost > 0 {
            let tok: u64 = self.tasks.iter().map(|t| t.tokens).sum();
            out.push_str(&format!(
                "\n\n\u{03a3} {} tok \u{00b7} {}",
                fmt_tok(tok),
                fmt_cost(cost)
            ));
        }
        if let Some(tl) = timeline_block(&self.spans()) {
            out.push_str("\n\n");
            out.push_str(&tl);
        }
        out
    }

    /// Per-task `(title, Some((start_ms, end_ms)))` offsets within the run,
    /// `None` when the task never started. A task still running at fold time
    /// (error-path fold) closes at "now" — an honest partial bar.
    fn spans(&self) -> Vec<(String, Option<(u64, u64)>)> {
        self.tasks
            .iter()
            .map(|t| {
                let span = t.started.map(|s| {
                    let start = s.duration_since(self.run_started).as_millis() as u64;
                    let end = match t.elapsed_ms {
                        Some(e) => start + e,
                        None => self.run_started.elapsed().as_millis() as u64,
                    };
                    (start, end.max(start))
                });
                (t.title.clone(), span)
            })
            .collect()
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

/// Micro-USD as dollars — 2 decimals once a cent is reached, 4 below it so
/// sub-cent task costs don't all collapse to `$0.00`. Shared by the live
/// block and the folded record.
pub(crate) fn fmt_cost(micros: u64) -> String {
    let usd = micros as f64 / 1_000_000.0;
    if micros >= 10_000 {
        format!("${usd:.2}")
    } else {
        format!("${usd:.4}")
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
