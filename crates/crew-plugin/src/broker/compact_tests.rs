//! Condition 5: transcript history nearing the budget is SUMMARIZED, not
//! dropped — an early load-bearing decision is still present in a late hop's
//! prompt via the compact block — and every failure mode falls back to the
//! old clipping, never an error.
use std::sync::{Arc, Mutex};

use super::*;
use crate::broker::{Adapter, Broker, Registry};

/// Records every prompt; replies from a script, repeating the last.
struct Capturing {
    name: String,
    replies: Vec<String>,
    idx: Mutex<usize>,
    calls: Arc<Mutex<Vec<String>>>,
}

impl Capturing {
    fn scripted(name: &str, replies: Vec<String>) -> (Box<dyn Adapter>, Arc<Mutex<Vec<String>>>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let agent = Box::new(Capturing {
            name: name.into(),
            replies,
            idx: Mutex::new(0),
            calls: Arc::clone(&calls),
        });
        (agent, calls)
    }
}

impl Adapter for Capturing {
    fn name(&self) -> &str {
        &self.name
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: std::time::Duration) -> Result<String, String> {
        self.calls.lock().unwrap().push(body.to_string());
        let mut i = self.idx.lock().unwrap();
        let r = self
            .replies
            .get(*i)
            .or_else(|| self.replies.last())
            .cloned()
            .unwrap_or_default();
        *i += 1;
        Ok(r)
    }
}

/// An accumulate-and-echo summarizer: honest about the mechanism under test
/// (retention + prepending), deterministic, and keyless. The head of its
/// accumulated input — which contains the FIRST overflow entries — is what
/// it returns, so an early token survives iterated re-summarization.
fn echo_summarizer() -> Summarize {
    let acc = Arc::new(Mutex::new(String::new()));
    Arc::new(move |req: &str| {
        let mut a = acc.lock().unwrap();
        if a.is_empty() {
            a.push_str(req);
        }
        Ok(clip(&a, 400))
    })
}

/// A 20-exchange ping-pong relay: the load-bearing decision (a distinctive
/// token) is planted in hop 1 and every later body is filler. Returns the
/// prompts the `claude` agent saw.
fn long_relay(summarizer: Option<Summarize>) -> Vec<String> {
    let mut replies = vec!["decision: the cipher key is blue-heron-42\n@next codex".to_string()];
    for n in 2..12 {
        replies.push(format!("step {n} done, nothing new\n@next codex"));
    }
    replies.push("wrapping up\n@done".to_string());
    let (claude, claude_calls) = Capturing::scripted("claude", replies);
    let codex_replies = (1..=12)
        .map(|n| format!("ack {n}, carry on\n@next claude"))
        .collect();
    let (codex, _) = Capturing::scripted("codex", codex_replies);
    let broker = Broker::new(
        Registry::new(vec![claude, codex]),
        60,
        std::time::Duration::from_secs(1),
    )
    .with_summarizer(summarizer);
    let mut hops = Vec::new();
    broker.run(
        "user",
        "claude",
        "keep relaying",
        "t1",
        &crate::broker::tick::noop_tick_emit(),
        &mut |h| hops.push(h),
    );
    let calls = claude_calls.lock().unwrap();
    calls.clone()
}

#[test]
fn an_early_decision_survives_into_a_late_prompt_via_the_summary_block() {
    let prompts = long_relay(Some(echo_summarizer()));
    let last = prompts.last().unwrap();
    assert!(
        last.contains("blue-heron-42"),
        "the hop-1 decision fell out of a late prompt:\n{last}"
    );
    assert!(
        last.contains("[compacted"),
        "the compact block must announce itself:\n{last}"
    );
}

/// The fence documenting the pre-compaction behavior this replaces: without
/// a summarizer the same decision falls off the 8-entry tail.
#[test]
fn without_a_summarizer_the_old_clipping_drops_the_early_decision() {
    let prompts = long_relay(None);
    let last = prompts.last().unwrap();
    assert!(!last.contains("blue-heron-42"), "{last}");
    assert!(!last.contains("[compacted"), "{last}");
}

#[test]
fn a_failing_summarizer_degrades_to_clipping_never_an_error() {
    let boom: Summarize = Arc::new(|_| Err("boom".into()));
    let prompts = long_relay(Some(boom));
    let last = prompts.last().unwrap();
    // Exactly the no-summarizer output: overflow clipped, no block, and the
    // relay itself completed (we got the full run's prompts).
    assert!(!last.contains("blue-heron-42"), "{last}");
    assert!(!last.contains("[compacted"), "{last}");
    assert_eq!(prompts.len(), 12, "the relay must run to completion");
}

/// The retained block is byte-bounded however verbose the model gets.
#[test]
fn the_running_summary_is_clipped_to_its_cap() {
    let verbose: Summarize = Arc::new(|_| Ok("x".repeat(50_000)));
    let mut c = Compactor::new(Some(verbose));
    let mut transcript: Vec<String> = (0..KEEP + 4).map(|n| format!("entry {n}")).collect();
    let out = c.tail(&mut transcript);
    let block = out.lines().next().unwrap();
    assert!(
        block.len() <= SUMMARY_CAP + 64,
        "summary block must stay bounded: {} bytes",
        block.len()
    );
}

// ── fold_log: the session log folds instead of halving ─────────────────────

fn seed_log() -> String {
    let mut log = String::from("user: the deploy password hint is kestrel-9\n");
    for n in 0..600 {
        log.push_str(&format!("coder: filler line {n} with routine chatter\n"));
    }
    log
}

#[test]
fn fold_log_retains_the_oldest_half_as_a_summary_header() {
    let log = seed_log();
    let cap = log.len(); // already over budget by construction of the call
    let folded = fold_log(&log, cap, Some(echo_summarizer()));
    assert!(
        folded.starts_with("[compacted earlier session:"),
        "{folded}"
    );
    assert!(
        folded.contains("kestrel-9"),
        "the early note must survive the fold: {folded}"
    );
    assert!(folded.len() <= cap, "{} > cap {}", folded.len(), cap);
    // The newest half is verbatim.
    assert!(folded.contains("filler line 599"), "{folded}");
}

#[test]
fn fold_log_without_a_summarizer_is_the_old_halving_byte_for_byte() {
    let log = seed_log();
    // The pre-compaction algorithm, verbatim.
    let mut cut = log.len() / 2;
    while cut < log.len() && log.as_bytes()[cut] != b'\n' {
        cut += 1;
    }
    let old = log[cut.min(log.len().saturating_sub(1)) + 1..].to_string();
    assert_eq!(fold_log(&log, log.len(), None), old);
    let boom: Summarize = Arc::new(|_| Err("boom".into()));
    assert_eq!(fold_log(&log, log.len(), Some(boom)), old);
}
