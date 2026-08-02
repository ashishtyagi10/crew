//! Summarize, never drop: broker-side context compaction. The relay's
//! transcript used to be blind fixed-window clipping (last [`KEEP`] entries,
//! `hop::transcript_tail`) and the session log halved itself by discarding
//! its oldest lines. Here the overflow is folded by ONE bounded model call
//! into a running compact block that is retained and prepended — the
//! broker-side mirror of the app's `chatcompact.rs` "compacted N earlier
//! messages" — so an early decision stays recoverable in a late prompt.
//! Keyless, mock, `CREW_INTENT=0`, or a failed call all fall back to the old
//! clipping: degraded context, never an error.
use std::sync::Arc;

use super::hop::transcript_tail;
use super::route::clip;

/// Transcript entries kept verbatim — the pre-compaction window, unchanged.
pub(crate) const KEEP: usize = 8;

/// Output-token ceiling for one summarization call (the same bounded-call
/// idiom as `intent::classify`, which supplies the plumbing).
pub(crate) const SUMMARY_MAX_TOKENS: u32 = 256;

/// Char budget for the RETAINED running summary. The block rides every later
/// hop's prompt, so it is clipped like any other context, never open-ended.
pub(crate) const SUMMARY_CAP: usize = 1200;

/// A summarization call: request in, summary text out.
pub(crate) type Summarize = Arc<dyn Fn(&str) -> Result<String, String> + Send + Sync>;

/// The live summarizer, when one may run — same gates as the classifier
/// (provider resolved, not mock, not `CREW_INTENT=0`), with its own token
/// ceiling. `None` means callers keep today's clipping.
pub(crate) fn live_summarizer() -> Option<Summarize> {
    let call = super::intent::live_call(SUMMARY_MAX_TOKENS)?;
    Some(Arc::new(call))
}

/// Whether a relayed body is worth a transcript line. An agent that hands off
/// with nothing but its control line contributes no information, but a stored
/// `"X → Y: "` entry still costs every later hop's prompt tokens.
pub(crate) fn keep_in_transcript(body: &str) -> bool {
    !body.trim().is_empty()
}

/// Whether `entry` would duplicate the immediately-preceding transcript entry
/// byte-for-byte — a consecutive repeat that costs prompt tokens for zero new
/// information.
pub(crate) fn is_dup(transcript: &[String], entry: &str) -> bool {
    transcript.last().is_some_and(|last| last == entry)
}

/// Per-thread compaction state: the running summary and how many transcript
/// entries it has folded so far.
pub(crate) struct Compactor {
    call: Option<Summarize>,
    summary: Option<String>,
    folded: usize,
}

impl Compactor {
    pub(crate) fn new(call: Option<Summarize>) -> Self {
        Self {
            call,
            summary: None,
            folded: 0,
        }
    }

    /// The bounded conversation context for the next hop. Entries beyond the
    /// [`KEEP`] window are folded into the running summary (and drained from
    /// `transcript`); the result is the compact block, then the verbatim
    /// tail. Without a summarizer — or when a call fails — nothing is
    /// drained and the overflow simply falls off the fixed tail: exactly the
    /// pre-compaction clipping, never an error.
    pub(crate) fn tail(&mut self, transcript: &mut Vec<String>) -> String {
        if transcript.len() > KEEP {
            if let Some(call) = &self.call {
                let cut = transcript.len() - KEEP;
                let req = request(self.summary.as_deref(), &transcript[..cut]);
                if let Ok(s) = call(&req) {
                    if !s.trim().is_empty() {
                        self.folded += cut;
                        self.summary = Some(clip(&s, SUMMARY_CAP));
                        transcript.drain(..cut);
                    }
                }
            }
        }
        let tail = transcript_tail(transcript);
        match &self.summary {
            Some(s) if self.folded > 0 => {
                format!("[compacted {} earlier messages: {s}]\n{tail}", self.folded)
            }
            _ => tail,
        }
    }
}

/// The summarization request: the running summary (so nothing already folded
/// is lost), then the overflow entries.
fn request(prev: Option<&str>, overflow: &[String]) -> String {
    format!(
        "Compress this agent conversation into one compact running summary \
         (short lines, at most ~150 words). Keep every decision, constraint, \
         file name and number a later step might depend on; drop pleasantries.\n\n\
         {}New messages:\n{}\n\nReply with the summary only.",
        prev.map(|p| format!("Summary so far:\n{p}\n\n"))
            .unwrap_or_default(),
        overflow.join("\n"),
    )
}

/// Fold an over-budget session log to fit `cap`: the oldest half is
/// summarized into a retained `[compacted earlier session: …]` header and
/// the newest half kept verbatim. No summarizer, a failed call, or a header
/// that would bust `cap` all fall back to dropping the oldest half — the
/// pre-compaction behavior, byte-for-byte.
pub(crate) fn fold_log(log: &str, cap: usize, call: Option<Summarize>) -> String {
    let cut = half_cut(log);
    let (old, new) = log.split_at(cut);
    if let Some(call) = call {
        if let Ok(s) = call(&request(None, &[old.to_string()])) {
            if !s.trim().is_empty() {
                let head = format!("[compacted earlier session: {}]\n", clip(&s, SUMMARY_CAP));
                if head.len() + new.len() <= cap {
                    return format!("{head}{new}");
                }
            }
        }
    }
    new.to_string()
}

/// The byte offset one past the first line boundary at/after the log's
/// midpoint — where the "oldest half" ends.
fn half_cut(log: &str) -> usize {
    let mut cut = log.len() / 2;
    while cut < log.len() && log.as_bytes()[cut] != b'\n' {
        cut += 1;
    }
    cut.min(log.len().saturating_sub(1)) + 1
}

#[cfg(test)]
#[path = "compact_tests.rs"]
mod tests;
