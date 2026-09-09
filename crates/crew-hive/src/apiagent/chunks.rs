//! The bus-facing end of a provider stream: one [`ChunkFn`] that publishes
//! reply text as `OutputDelta` and reasoning as `ThoughtDelta`, plus the
//! catch-up for a provider that does not stream at all.
//!
//! Streaming is advisory (see `HiveEvent::OutputDelta`), so a reply that
//! never streamed loses nothing — its `OutputChunk` carries the whole text.
//! Reasoning has no such settled twin: nothing but the deltas ever carries
//! it to a subscriber. So a completion whose `thought` arrived in one piece
//! (Anthropic's `thinking` block, a non-streamed `reasoning_content`) is
//! published as one delta after the fact — but only when NO fragment of it
//! streamed, or the pane would show the same working twice.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::provider::{Chunk, ChunkFn, Completion};

pub(super) struct ChunkSink {
    pub on_chunk: ChunkFn,
    streamed_thought: Arc<AtomicBool>,
    bus: EventBus,
    agent: AgentId,
}

impl ChunkSink {
    pub(super) fn new(bus: EventBus, agent: AgentId) -> Self {
        let streamed_thought = Arc::new(AtomicBool::new(false));
        let (delta_bus, delta_agent, seen) = (bus.clone(), agent.clone(), streamed_thought.clone());
        let on_chunk: ChunkFn = Arc::new(move |c: Chunk<'_>| match c {
            Chunk::Text(s) => delta_bus.publish(HiveEvent::OutputDelta {
                agent: delta_agent.clone(),
                text: s.to_string(),
            }),
            Chunk::Thought(s) => {
                seen.store(true, Ordering::SeqCst);
                delta_bus.publish(HiveEvent::ThoughtDelta {
                    agent: delta_agent.clone(),
                    text: s.to_string(),
                });
            }
        });
        Self {
            on_chunk,
            streamed_thought,
            bus,
            agent,
        }
    }

    /// One round's completion landed: publish reasoning that never streamed.
    /// Resets the flag, so the next round of a tool loop is judged on its own.
    pub(super) fn settle(&self, c: &Completion) {
        let streamed = self.streamed_thought.swap(false, Ordering::SeqCst);
        if !streamed && !c.thought.is_empty() {
            self.bus.publish(HiveEvent::ThoughtDelta {
                agent: self.agent.clone(),
                text: c.thought.clone(),
            });
        }
    }
}
