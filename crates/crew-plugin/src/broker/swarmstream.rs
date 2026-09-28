//! The swarm's closing answer, typed out as it is written.
//!
//! Every other answer in the pane streams: the relay's replies, each worker's,
//! the thinking fold. The lead's closing answer (`swarmanswer`) — usually the
//! longest thing a swarm turn writes — was one blocking call: the workers'
//! rows settled, smith sat "thinking" through the whole generation (seconds,
//! for a 2,048-token answer), and then all of it landed at once. It is now the
//! provider's streaming call. Its text goes out as the lead's `Delta`s, paced
//! through the same [`TextGate`] as every other stream, and the whole answer
//! still lands as the lead's one `Message`, which the app pairs with the live
//! card by sender name and puts in its place — so whatever the gate held back
//! is not lost. All of it goes out under the name smith answers under
//! (`relay::SMITH_ANSWERS`), not his bare chrome name: the live card is drawn
//! from its first word as the reply it becomes, not as a muted status line
//! that turns into one when it lands. Reasoning, when the provider tells it
//! apart, is the lead's `Thought`, never answer text. A provider that cannot
//! stream (the trait's default hands the call to `complete`) sends no pieces,
//! and the pane sees the one `Message` it always did.
//!
//! No `streamhold`: it holds back an `@`-routing line mid-stream, and the
//! lead's answer has no routing grammar — the user is its only reader.
use std::sync::Arc;
use std::time::Instant;

use crew_hive::{Chunk, ChunkFn, Completion, CompletionRequest, Provider};

use crate::broker::relay::SMITH_ANSWERS;
use crate::broker::tick::{text_streaming_enabled, TextGate};
use crate::protocol::PluginEvent;

/// The closing call: the brief in, the whole answer out (marked when the
/// ceiling cut it, `cutoff::marked`), and each paced piece of it handed to
/// the sink on the way — reply text or reasoning, as the provider told them
/// apart. The judge keeps `swarmanswer::SynthFn`, which has no sink: a
/// verdict is one line of chrome, not an answer to watch being written.
pub(crate) type AnswerFn = dyn Fn(&str, &mut dyn FnMut(Chunk<'_>)) -> Result<String, String>;

/// The closing call on `provider`: `max_tokens` of `model`, bounded like a
/// relay agent's call (`session::call_timeout`, 3 min), not by the 30 s
/// one-shot bound the blocking call had — 2,048 tokens on qwen-max run past
/// 30 s, and a streamed answer cut there is watched half-written and then
/// replaced by an error. The provider's read timeout still ends a SILENT one.
pub(super) fn over(provider: Arc<dyn Provider>, model: String, max_tokens: u32) -> Box<AnswerFn> {
    Box::new(move |brief: &str, sink: &mut dyn FnMut(Chunk<'_>)| {
        let req = CompletionRequest {
            model: model.clone(),
            prompt: brief.to_string(),
            max_tokens,
            ..Default::default()
        };
        let c = streamed(&provider, req, sink)?;
        Ok(crate::broker::cutoff::marked(&c.text, c.truncated))
    })
}

/// Run `call` on `brief`, saying each piece as the lead's live card as it
/// comes: a `Delta` of the answer, a `Thought` of its working — under the
/// name the answer's `Message` goes out under, which is how the app knows
/// the one settles the other. An emit that fails (the pane gone) stops the
/// saying, not the call, and comes back once the call is over, the way every
/// other emit failure in a run does.
pub(super) fn said(
    call: &AnswerFn,
    brief: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<Result<String, String>> {
    let mut failed = None;
    let reply = call(brief, &mut |piece| {
        if failed.is_some() {
            return;
        }
        let agent = SMITH_ANSWERS.to_string();
        let ev = match piece {
            Chunk::Text(t) => PluginEvent::Delta {
                agent,
                text: t.to_string(),
                sub: false,
            },
            Chunk::Thought(t) => PluginEvent::Thought {
                agent,
                text: t.to_string(),
            },
        };
        failed = emit(ev).err();
    });
    failed.map_or(Ok(reply), Err)
}

/// A piece as it crossed from the provider, owned: the provider's callback
/// must be `Send + Sync` and the pane's emitter is neither, so pieces cross
/// a channel to this thread and are said here.
enum Piece {
    Text(String),
    Thought(String),
}

/// One streamed completion, its pieces paced through a [`TextGate`] and
/// handed to `sink` on THIS thread while the call runs. Each is stamped when
/// the provider delivered it, not when it was drained, so the pacing is the
/// stream's own. Text gets no end flush (the settled `Message` carries all
/// of it a moment later, as `tick::hop_texter` says); reasoning does — at the
/// answer's first word, and at the end — since nothing heals a thought the
/// gate swallowed. `CREW_STREAM_TEXT=0` says nothing mid-call, like every
/// other stream.
fn streamed(
    provider: &Arc<dyn Provider>,
    req: CompletionRequest,
    sink: &mut dyn FnMut(Chunk<'_>),
) -> Result<Completion, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let start = Instant::now();
    let on_chunk: ChunkFn = Arc::new(move |c: Chunk<'_>| {
        let piece = match c {
            Chunk::Text(t) => Piece::Text(t.to_string()),
            Chunk::Thought(t) => Piece::Thought(t.to_string()),
        };
        // The receiver outlives the call, so a send cannot fail.
        let _ = tx.send((start.elapsed().as_millis() as u64, piece));
    });
    let call = provider.complete_streaming(req, on_chunk);
    let (enabled, mut gate) = (text_streaming_enabled(), TextGate::new());
    let mut pass = |(at, piece): (u64, Piece)| match piece {
        _ if !enabled => {}
        Piece::Text(t) => {
            // The working the gap still held goes first: a thought that
            // landed under its answer would read as thinking about nothing.
            if let Some(tail) = gate.flush_thought() {
                sink(Chunk::Thought(&tail));
            }
            if let Some(p) = gate.push(&t, at) {
                sink(Chunk::Text(&p));
            }
        }
        Piece::Thought(t) => {
            if let Some(p) = gate.push_thought(&t, at) {
                sink(Chunk::Thought(&p));
            }
        }
    };
    let done = rt.block_on(async {
        // Made here, not above: the timer needs the runtime it runs on.
        let call = tokio::time::timeout(crate::broker::session::call_timeout(), call);
        tokio::pin!(call);
        loop {
            // Pieces first: one that is waiting is said before the call's end.
            tokio::select! {
                biased;
                Some(piece) = rx.recv() => pass(piece),
                done = &mut call => break done,
            }
        }
    });
    // Whatever arrived in the same poll that finished the call.
    while let Ok(piece) = rx.try_recv() {
        pass(piece);
    }
    if let Some(tail) = gate.flush_thought() {
        sink(Chunk::Thought(&tail));
    }
    match done {
        Ok(Ok(c)) => Ok(c),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err(format!(
            "timed out after {:?}",
            crate::broker::session::call_timeout()
        )),
    }
}

#[cfg(test)]
#[path = "swarmstream_tests.rs"]
mod tests;
