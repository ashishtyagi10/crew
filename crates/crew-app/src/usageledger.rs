//! Crew's own usage history, for the footer's Claude-style rolling windows.
//! A window opens at the first request after the previous window expired
//! (Claude session semantics) — a 5-hour and a 7-day block — and the footer
//! shows time left + budget spent for each. Entries persist as JSON lines in
//! `usage.jsonl` beside `config.toml`, pruned past 7 days on load. The
//! process-wide singleton aggregates across panes (the GUI is one process;
//! brokers are per-pane children and would race on the file).
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

// `windows()`/`Windows`/`WindowStat` are consumed by the footer in Task 6;
// until then these are only reachable from tests, so allow the dead-code
// warnings rather than leaving the build noisy.
#[allow(dead_code)]
pub(crate) const FIVE_H_MS: u64 = 5 * 3_600_000;
pub(crate) const SEVEN_D_MS: u64 = 7 * 24 * 3_600_000;

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Entry {
    pub ts_ms: u64,
    pub tok_in: u64,
    pub tok_out: u64,
    pub cost_microusd: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WindowStat {
    /// Milliseconds until this block rolls over.
    pub left_ms: u64,
    /// Tokens (in+out) spent inside the block so far.
    pub spent: u64,
    pub budget: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct Windows {
    pub five_h: Option<WindowStat>,
    pub seven_d: Option<WindowStat>,
}

#[derive(Default)]
pub(crate) struct Ledger {
    pub(crate) entries: Vec<Entry>,
    #[allow(dead_code)]
    pub(crate) budget_5h: u64,
    #[allow(dead_code)]
    pub(crate) budget_7d: u64,
}

impl Ledger {
    /// The open `span_ms` block at `now_ms`, if any. Blocks chain: the first
    /// entry opens one; an entry past a block's end opens the next at its own
    /// timestamp (not at the boundary), so idle gaps don't tick windows over.
    pub(crate) fn window(&self, now_ms: u64, span_ms: u64, budget: u64) -> Option<WindowStat> {
        let mut start: Option<u64> = None;
        for e in &self.entries {
            match start {
                None => start = Some(e.ts_ms),
                Some(s) if e.ts_ms >= s + span_ms => start = Some(e.ts_ms),
                Some(_) => {}
            }
        }
        let s = start?;
        if now_ms >= s + span_ms {
            return None; // last block expired with no use since
        }
        let spent = self
            .entries
            .iter()
            .filter(|e| e.ts_ms >= s)
            .map(|e| e.tok_in + e.tok_out)
            .sum();
        Some(WindowStat {
            left_ms: s + span_ms - now_ms,
            spent,
            budget,
        })
    }

    #[allow(dead_code)]
    pub(crate) fn windows(&self, now_ms: u64) -> Windows {
        Windows {
            five_h: self.window(now_ms, FIVE_H_MS, self.budget_5h),
            seven_d: self.window(now_ms, SEVEN_D_MS, self.budget_7d),
        }
    }

    /// Drop entries older than the 7d horizon — nothing renders them again.
    pub(crate) fn prune(&mut self, now_ms: u64) {
        let floor = now_ms.saturating_sub(SEVEN_D_MS);
        self.entries.retain(|e| e.ts_ms >= floor);
    }
}

/// `usage.jsonl` beside the config file.
fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("usage.jsonl"))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

static LEDGER: Mutex<Option<Ledger>> = Mutex::new(None);

/// Load the ledger and set budgets. Call once at startup, after config load.
pub(crate) fn init(budget_5h: u64, budget_7d: u64) {
    let mut l = Ledger {
        entries: Vec::new(),
        budget_5h,
        budget_7d,
    };
    if !cfg!(test) {
        if let Some(p) = path() {
            if let Ok(text) = std::fs::read_to_string(&p) {
                l.entries = text
                    .lines()
                    .filter_map(|line| serde_json::from_str(line).ok())
                    .collect();
            }
            l.prune(now_ms());
            // Rewrite pruned so the file doesn't grow without bound.
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let body: String = l
                .entries
                .iter()
                .filter_map(|e| serde_json::to_string(e).ok())
                .map(|s| s + "\n")
                .collect();
            let _ = std::fs::write(&p, body);
        }
    }
    *LEDGER.lock().unwrap_or_else(|e| e.into_inner()) = Some(l);
}

/// Record one turn's usage: in-memory always, appended to disk outside tests.
pub(crate) fn record(tok_in: u64, tok_out: u64, cost_microusd: u64) {
    if tok_in == 0 && tok_out == 0 {
        return; // mock/CLI backends report no usage — nothing to window
    }
    let e = Entry {
        ts_ms: now_ms(),
        tok_in,
        tok_out,
        cost_microusd,
    };
    let mut guard = LEDGER.lock().unwrap_or_else(|e| e.into_inner());
    guard.get_or_insert_with(Ledger::default).entries.push(e);
    if cfg!(test) {
        return;
    }
    if let Some(p) = path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let (Ok(line), Ok(mut f)) = (
            serde_json::to_string(&e),
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&p),
        ) {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// The current rolling windows, for the footer. Cheap: a scan over ≤7d of
/// per-turn entries under a mutex — fine on the render path.
#[allow(dead_code)]
pub(crate) fn windows(now_ms: u64) -> Windows {
    let guard = LEDGER.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .map(|l| l.windows(now_ms))
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "usageledger_tests.rs"]
mod tests;
