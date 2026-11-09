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
/// idiom as `intent::classify`, which supplies the plumbing). The summary
/// itself is ~300 tokens ([`SUMMARY_CAP`] chars), but it runs at the
/// model's own effort and a Claude 5.x model pays its thinking out of this
/// (as `advise::MAX_TOKENS` does); 256 left it nothing to answer with.
pub(crate) const SUMMARY_MAX_TOKENS: u32 = 1024;

/// The summary's effort: the model's default, NOT the one-shots' low
/// (`intent::classify::ONESHOT_EFFORT`). A route or a verdict is thrown away
/// after one use; this summary replaces the history every later hop works
/// from, and a thin one sends the model back to redo finished work.
pub(crate) const SUMMARY_EFFORT: Option<crew_hive::Effort> = None;

/// The labels a summary's lines carry. What was done and what was ruled out
/// are what a model redoes or retries when they are missing after a
/// compaction; `open:` is what it should do instead.
const LABELS: [&str; 3] = ["done:", "ruled out:", "open:"];

/// Char budget for the RETAINED running summary. The block rides every later
/// hop's prompt, so it is clipped like any other context, never open-ended.
pub(crate) const SUMMARY_CAP: usize = 1200;

/// A summarization call: request in, summary text out.
pub(crate) type Summarize = Arc<dyn Fn(&str) -> Result<String, String> + Send + Sync>;

/// The live summarizer, when one may run — same gates as the classifier
/// (provider resolved, not mock, not `CREW_INTENT=0`), with its own token
/// ceiling. `None` means callers keep today's clipping.
pub(crate) fn live_summarizer() -> Option<Summarize> {
    let (provider, model) = super::intent::live_provider_at(crew_hive::ModelTier::Cheap)?;
    Some(summarizer_on(provider, model))
}

/// The summarizer on a given provider: the one-shot plumbing at the
/// summary's own ceiling and effort ([`SUMMARY_EFFORT`]).
pub(crate) fn summarizer_on(provider: Arc<dyn crew_hive::Provider>, model: String) -> Summarize {
    use super::intent::classify::{complete_once, ONESHOT_TIMEOUT};
    Arc::new(move |req: &str| {
        let (max, effort) = (SUMMARY_MAX_TOKENS, SUMMARY_EFFORT);
        complete_once(&provider, &model, req, max, ONESHOT_TIMEOUT, effort).map(|c| c.text)
    })
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
                        self.summary = Some(fit(&s, SUMMARY_CAP));
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
/// is lost), then the overflow entries. It asks for [`LABELS`] lines, not
/// prose: a summary of "decisions and file names" kept the plan but lost
/// which edits had landed and which approach had already failed, and a late
/// hop re-did the one and retried the other.
pub(crate) fn request(prev: Option<&str>, overflow: &[String]) -> String {
    format!(
        "Compress this agent conversation into one running summary that a later \
         step resumes from without redoing or retrying anything (at most ~150 \
         words). Write only lines starting with these labels, as many of each \
         as needed:\n\
         done: what was finished: each file changed, each command or tool run \
         (by its tool name) and its result\n\
         ruled out: each approach that failed or was rejected, and why\n\
         open: what is still left to do\n\
         Keep every decision, constraint, file name and number a later step \
         might depend on; drop pleasantries. Carry forward every line of the \
         summary so far that still holds.\n\n\
         {}New messages:\n{}\n\nReply with the summary lines only.",
        prev.map(|p| format!("Summary so far:\n{p}\n\n"))
            .unwrap_or_default(),
        overflow.join("\n"),
    )
}

/// The reply bounded to `cap` chars on one line. A summary that fits is
/// [`clip`]ped as before; one that does not keeps its labelled lines, each
/// cut to an even share, over chatter: a plain clip drops the tail, and the
/// tail is where `ruled out:` and `open:` sit.
pub(crate) fn fit(summary: &str, cap: usize) -> String {
    let flat = clip(summary, cap);
    if !flat.ends_with('…') {
        return flat;
    }
    let kept: Vec<&str> = summary.lines().filter(|l| labelled(l)).collect();
    if kept.is_empty() {
        return flat;
    }
    // Each clipped line can grow by its `…` and the joining space.
    let share = (cap / kept.len()).saturating_sub(2).max(1);
    let lines: Vec<String> = kept.iter().map(|l| clip(l, share)).collect();
    lines.join(" ")
}

/// Whether `line` opens with one of [`LABELS`], past a list bullet.
fn labelled(line: &str) -> bool {
    let l = line.trim_start_matches(|c: char| c == '-' || c == '*' || c.is_whitespace());
    LABELS.iter().any(|label| {
        l.get(..label.len())
            .is_some_and(|h| h.eq_ignore_ascii_case(label))
    })
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
                let head = format!("[compacted earlier session: {}]\n", fit(&s, SUMMARY_CAP));
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

#[cfg(test)]
#[path = "compactkeep_tests.rs"]
mod keep_tests;
