//! One API agent's call outside the harness: the request to its model, made
//! whole and raced against the task's /stop, and what the pane is fed while
//! the reply streams.
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crew_hive::{Completion, CompletionRequest};

use super::{ApiAdapter, MAX_TOKENS};
use crate::broker::adapter::HopStream;
use crate::broker::cancelscope;

impl ApiAdapter {
    /// `body` as one request to this agent's model, made whole
    /// (`crew_hive::provider::cutoff`) with `on_chunk` fed as it streams, and
    /// raced against `timeout` and the task's /stop (`cancelscope`): Esc ends
    /// the wait. The reply, or why there is none in the words the error card
    /// shows.
    pub(super) fn ask(
        &self,
        body: &str,
        timeout: Duration,
        on_chunk: Option<crew_hive::ChunkFn>,
    ) -> Result<Completion, String> {
        let req = CompletionRequest {
            model: self.model.clone(),
            system: self.system.clone(),
            prompt: body.to_string(),
            max_tokens: MAX_TOKENS,
            ..Default::default()
        };
        let provider = Arc::clone(&self.provider);
        let fut = crew_hive::provider::cutoff::whole(provider, req, on_chunk, timeout);
        let raced = cancelscope::unless(cancelscope::current(), async move {
            // Built inside the runtime: a timer made outside it has no reactor.
            tokio::time::timeout(timeout, fut).await
        });
        match self.rt.block_on(raced) {
            None => Err(cancelscope::STOPPED.to_string()),
            Some(Ok(Ok(c))) => Ok(c),
            Some(Ok(Err(e))) => Err(e.to_string()),
            Some(Err(_)) => Err(format!(
                "{}: api call timed out after {timeout:?} (raise CREW_BROKER_TIMEOUT_MS?)",
                self.model
            )),
        }
    }
}

/// What a streaming reply feeds `stream`: each raw text fragment and a
/// running chars/4 OUTPUT-token estimate, each fragment of reasoning; and
/// whether any reasoning came that way, set once it has.
pub(super) fn chunks(stream: &HopStream) -> (crew_hive::ChunkFn, Arc<AtomicBool>) {
    let chars = Arc::new(AtomicU64::new(0));
    let counter = chars.clone();
    let on_tokens = Arc::clone(&stream.on_tokens);
    let on_text = Arc::clone(&stream.on_text);
    let on_thought = Arc::clone(&stream.on_thought);
    let streamed_thought = Arc::new(AtomicBool::new(false));
    let seen_thought = streamed_thought.clone();
    let on_chunk: crew_hive::ChunkFn = Arc::new(move |c: crew_hive::Chunk<'_>| {
        let s = match c {
            crew_hive::Chunk::Text(s) => s,
            crew_hive::Chunk::Thought(s) => {
                seen_thought.store(true, Ordering::SeqCst);
                if !s.is_empty() {
                    on_thought(s);
                }
                return;
            }
        };
        // Text first: it is what the user sees, and the token estimate
        // must never delay it.
        on_text(s);
        // Unicode chars, not bytes — byte counts over-report CJK ~3×
        // (same convention as the provider-side chars/4 estimators).
        let n = s.chars().count() as u64;
        let total = counter.fetch_add(n, Ordering::SeqCst) + n;
        on_tokens(total / 4);
    });
    (on_chunk, streamed_thought)
}
