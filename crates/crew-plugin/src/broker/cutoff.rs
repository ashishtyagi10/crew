//! A relay reply that ran into the output-token ceiling, finished.
//!
//! Nothing used to notice the ceiling. A long answer reached the pane ending
//! mid-sentence as if that were all of it, and a `sys:write_file` of a real
//! file, or a `sys:edit` with a big `new` string, stopped halfway through its
//! JSON and failed as "not valid JSON" with nothing saying why. Every
//! provider now reports the stop (`Completion::truncated`), and a cut reply
//! is continued ONCE — the partial goes back as the model's own turn, then a
//! user turn asks for the rest — and the halves are joined, the way Claude
//! Code carries on after a `max_tokens` stop. Once, not until done: a reply
//! that fills the ceiling twice is a runaway, and a third bill for it is
//! worse than saying so. What stays cut says so in the reply ([`CUT_OFF`]),
//! so nobody mistakes the end of the budget for the end of the answer.
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crew_hive::{ChunkFn, Completion, CompletionRequest, Provider, ProviderError, Turn};

/// The last line of a reply that stayed cut off.
pub(crate) const CUT_OFF: &str = "\u{2026} (cut off at the token limit)";

/// The follow-up turn. "No preamble, no repetition": asked to go on, a model
/// otherwise opens with "Continuing from where I left off:" or re-sends the
/// paragraph it was in, and either lands in the middle of the joined text —
/// or of a JSON string.
const CONTINUE: &str = "Your reply was cut off at the token limit. Continue exactly \
where it stopped \u{2014} no preamble, no repetition.";

/// Room the continuation leaves under the caller's own deadline, so that it
/// is the continuation's timeout that fires — keeping the first half, marked
/// — and not the caller's, which would throw the first half away too.
const GRACE: Duration = Duration::from_secs(1);

type Call = Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>>;

/// `text` with [`CUT_OFF`] under it when the reply was cut, else as it is.
pub(crate) fn marked(text: &str, truncated: bool) -> String {
    if !truncated {
        return text.to_string();
    }
    match text.trim_end() {
        "" => CUT_OFF.to_string(),
        body => format!("{body}\n\n{CUT_OFF}"),
    }
}

/// The same request again, with the partial as the model's own turn and
/// [`CONTINUE`] as the user's. The partial goes back verbatim, trailing space
/// and all: the rest has to fit onto exactly what was said.
fn continuation(req: &CompletionRequest, partial: &str) -> CompletionRequest {
    let mut next = req.clone();
    next.turns.push(Turn::Assistant {
        text: partial.to_string(),
        calls: Vec::new(),
    });
    next.turns.push(Turn::User(CONTINUE.to_string()));
    next
}

/// One reply from `provider`, continued once if the ceiling cut it — streamed
/// through `on_chunk` when there is one, the continuation too, so the reply
/// keeps typing across the seam. `budget` is the caller's own timeout: the
/// continuation gets what is left of it, less [`GRACE`], and one that fails
/// or runs out leaves the first half standing, marked. A reply cut before it
/// said anything (a reasoning model that spent the ceiling thinking) is not
/// continued: there is nothing to continue FROM.
pub(crate) async fn whole(
    provider: Arc<dyn Provider>,
    req: CompletionRequest,
    on_chunk: Option<ChunkFn>,
    budget: Duration,
) -> Result<Completion, ProviderError> {
    let start = Instant::now();
    let first = call(&provider, req.clone(), on_chunk.clone()).await?;
    if !first.truncated || first.text.trim().is_empty() {
        return Ok(joined(first, None));
    }
    let left = budget.saturating_sub(start.elapsed()).saturating_sub(GRACE);
    let rest = call(&provider, continuation(&req, &first.text), on_chunk);
    let rest = tokio::time::timeout(left, rest)
        .await
        .ok()
        .and_then(Result::ok);
    Ok(joined(first, rest))
}

fn call(provider: &Arc<dyn Provider>, req: CompletionRequest, on_chunk: Option<ChunkFn>) -> Call {
    match on_chunk {
        Some(f) => provider.complete_streaming(req, f),
        None => provider.complete(req),
    }
}

/// The halves as one reply: text end to end, and usage and cost summed —
/// both requests were billed. (The input count doubles as the pane's
/// context-fill reading, which over-reads by one prompt for this one hop;
/// the bill is the number worth keeping right.) Cut is whatever the LAST
/// half was, and a cut reply's text ends with [`CUT_OFF`].
fn joined(first: Completion, rest: Option<Completion>) -> Completion {
    let mut c = first;
    if let Some(r) = rest {
        c.text.push_str(&r.text);
        c.input_tokens = c.input_tokens.saturating_add(r.input_tokens);
        c.cached_input_tokens = c.cached_input_tokens.saturating_add(r.cached_input_tokens);
        c.cache_write_tokens = c.cache_write_tokens.saturating_add(r.cache_write_tokens);
        c.output_tokens = c.output_tokens.saturating_add(r.output_tokens);
        if c.model.is_empty() {
            c.model = r.model;
        }
        c.cost_microusd = c.cost_microusd.saturating_add(r.cost_microusd);
        if !r.thought.is_empty() {
            if !c.thought.is_empty() {
                c.thought.push('\n');
            }
            c.thought.push_str(&r.thought);
        }
        c.calls.extend(r.calls);
        c.truncated = r.truncated;
    }
    if c.truncated {
        c.text = marked(&c.text, true);
    }
    c
}

#[cfg(test)]
#[path = "cutoff_tests.rs"]
mod tests;
