//! A reply whose stream broke partway, picked up where it stopped; a reply
//! with nothing in it, asked for again.
//!
//! A stream that died after the model had started talking failed the turn,
//! and the words already on screen went with it. The provider cannot help
//! there: it asks again only while nothing has been shown, because a fresh
//! answer would say again what the user already read. Here what was shown
//! goes back as the model's own turn, a user turn says it was interrupted,
//! and the rest is joined on — the shape `cutoff` gives a reply that ran
//! into the token ceiling. A reply with no words and no calls ended the turn
//! as "empty reply", though the same request asked again is usually
//! answered. Three calls at most: a host that breaks or blanks one request
//! three times running is not going to answer it, and a failure that fails
//! the same way every time (a missing or rejected key) is not asked again.
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::provider::{Chunk, ChunkFn, Completion, CompletionRequest, Provider, ProviderError};

use super::{call, continuation, summed, GRACE};

/// Calls one reply may take, the first included.
const TRIES: usize = 3;

/// The follow-up turn after a broken stream: "no preamble, no repetition"
/// for the reason `CONTINUE` gives, and the words it stopped on, so the join
/// keeps its space: told only to continue, qwen-max picked up "from the" with
/// "two branches" and the pane read "thetwo" (live probe, 2026-10-22).
fn resume_line(shown: &str) -> String {
    let at = shown
        .char_indices()
        .rev()
        .nth(TAIL - 1)
        .map_or(0, |(i, _)| i);
    format!(
        "Your reply was interrupted after: \u{ab}{}\u{bb}. Continue from the very next \
         character \u{2014} if a new word begins there, start with its space. No preamble, \
         no repetition.",
        &shown[at..]
    )
}

/// Characters of the broken reply's end quoted back to the model.
const TAIL: usize = 40;

/// One reply from `provider`, streamed through `on_chunk` when there is one.
/// A stream that broke after saying something, on a failure that passes
/// (`ProviderError::is_transient`), is continued from what it said, and a
/// pick-up answered with nothing means nothing was left to say. An empty
/// reply is asked again. A failure before the first word stands: the
/// provider has already retried it and walked its fallbacks. Each try after
/// the first gets what is left of `budget`, less [`GRACE`], and when the
/// tries or the time run out the reply is what it was without them — the
/// stream's error, or the empty reply. A broken try's usage is lost with it:
/// the provider never got to report it.
pub(super) async fn reply(
    provider: &Arc<dyn Provider>,
    req: &CompletionRequest,
    on_chunk: Option<ChunkFn>,
    budget: Duration,
) -> Result<Completion, ProviderError> {
    let start = Instant::now();
    let mut shown = String::new();
    let mut blanks = Completion::default();
    let mut broke = None;
    for n in 0..TRIES {
        let left = budget.saturating_sub(start.elapsed()).saturating_sub(GRACE);
        if n > 0 && left.is_zero() {
            break;
        }
        let ask = if shown.is_empty() {
            req.clone()
        } else {
            continuation(req, &shown, &resume_line(&shown))
        };
        let heard = Arc::new(Mutex::new(String::new()));
        let fut = call(provider, ask, on_chunk.clone().map(|f| tee(f, &heard)));
        let got = match n {
            0 => fut.await,
            _ => match tokio::time::timeout(left, fut).await {
                Ok(got) => got,
                Err(_) => break,
            },
        };
        let heard = std::mem::take(&mut *heard.lock().unwrap_or_else(|e| e.into_inner()));
        match got {
            Ok(c) if shown.is_empty() && blank(&c) => blanks = summed(blanks, c),
            Ok(c) => {
                blanks.text = shown;
                return Ok(summed(blanks, c));
            }
            Err(e) if e.is_transient() && !heard.trim().is_empty() => {
                shown.push_str(&heard);
                broke = Some(e);
            }
            Err(e) => return Err(e),
        }
    }
    broke.map_or(Ok(blanks), Err)
}

/// Nothing said and nothing called. A reply cut before its first word is
/// not blank here: the same ceiling would cut it again, and `cutoff` marks it.
fn blank(c: &Completion) -> bool {
    c.text.trim().is_empty() && c.calls.is_empty() && !c.truncated
}

/// `f`, with every word it is handed also kept in `heard` — what the user
/// has read if the stream breaks. Reasoning is passed on, not kept: it never
/// goes back to the model as its own words.
fn tee(f: ChunkFn, heard: &Arc<Mutex<String>>) -> ChunkFn {
    let heard = Arc::clone(heard);
    Arc::new(move |c: Chunk<'_>| {
        if let Chunk::Text(s) = c {
            heard.lock().unwrap_or_else(|e| e.into_inner()).push_str(s);
        }
        f(c);
    })
}

#[cfg(test)]
#[path = "resume_tests.rs"]
mod tests;
