//! The model's REASONING in the crew pane: every `PluginEvent::Thought` an
//! agent streams lands in that agent's [`Live`] buffer, drawn as a block
//! above its streaming card (`∴ thinking · 3s` and the last few lines of the
//! working) — and when the agent's reply settles, the buffer becomes a
//! [`ThoughtBlock`] anchored above that reply, folded to one row
//! (`▸ thought for 4.2 s · 812 chars`) until clicked. This module is the
//! model; the rows and their placement are `chatthoughtview`, the click
//! plumbing `chatthoughtfold` — the same split as the tool block
//! (`chattool`), which is the precedent for a live, collapsible block that
//! sits above a reply.
//!
//! Thoughts never enter the transcript (`messages`): the working is not
//! what the model SAID, so `/export`, the session log and the swarm fold
//! stay correct by construction rather than each filtering it out — and a
//! reply that reads its own scratch back as its words is exactly the bug
//! the separate event exists to prevent.
use crate::chat::ChatPane;
use crate::chatflow::stream_key;

/// Rows of the working the live block shows — the tail, as it arrives.
pub(crate) const LIVE_ROWS: usize = 4;
/// Rows an opened thought shows before `… +N lines`.
pub(crate) const THOUGHT_ROWS: usize = 40;

/// One agent's reasoning, still arriving.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Live {
    /// The name the agent's cards go by (`stream_key`).
    pub agent: String,
    pub text: String,
    /// When the first fragment landed — the block's clock.
    pub since_ms: u64,
    /// When the latest one did — the span the settled row reports.
    pub last_ms: u64,
}

/// A finished thought, folded above the reply it led to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ThoughtBlock {
    pub agent: String,
    /// The stamp of the settled reply the block sits above; `None` for one
    /// the run abandoned, which stands as its own thin card instead.
    pub anchor: Option<String>,
    pub text: String,
    /// First fragment to last, in ms. 0 for a thought that arrived whole.
    pub ms: u64,
    pub expanded: bool,
}

/// The pane's thoughts: the live buffers and the settled blocks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Thoughts {
    pub live: Vec<Live>,
    pub settled: Vec<ThoughtBlock>,
}

/// A pane with no thoughts — what a `View` built without one reads.
pub(crate) static EMPTY: Thoughts = Thoughts {
    live: Vec::new(),
    settled: Vec::new(),
};

impl Thoughts {
    /// One fragment landed: append it to `agent`'s buffer, opening one on
    /// the first.
    pub(crate) fn absorb(&mut self, agent: &str, text: &str, now_ms: u64) {
        match self.live.iter_mut().find(|l| l.agent == agent) {
            Some(l) => {
                l.text.push_str(text);
                l.last_ms = now_ms;
            }
            None => self.live.push(Live {
                agent: agent.to_string(),
                text: text.to_string(),
                since_ms: now_ms,
                last_ms: now_ms,
            }),
        }
    }

    /// The buffer `agent` is filling, if any — the tests' view of it; the
    /// frame reads `live` by index through `chatthoughtseat`.
    #[cfg(test)]
    pub(crate) fn live_of(&self, agent: &str) -> Option<&Live> {
        self.live.iter().find(|l| l.agent == agent)
    }

    /// `agent`'s reply settled as the card stamped `ts`: its buffer becomes
    /// the block anchored above that card. A reply with no thought leaves
    /// nothing behind.
    pub(crate) fn settle(&mut self, agent: &str, ts: &str, _now_ms: u64) {
        let Some(i) = self.live.iter().position(|l| l.agent == agent) else {
            return;
        };
        let l = self.live.remove(i);
        self.settled.push(ThoughtBlock {
            agent: l.agent,
            anchor: Some(ts.to_string()),
            text: l.text,
            ms: l.last_ms.saturating_sub(l.since_ms),
            expanded: false,
        });
    }

    /// The run is over: every buffer still live becomes an unanchored block —
    /// kept, because a thought that led to no reply is still what happened.
    pub(crate) fn abandon(&mut self, _now_ms: u64) {
        for l in self.live.drain(..) {
            self.settled.push(ThoughtBlock {
                agent: l.agent,
                anchor: None,
                text: l.text,
                ms: l.last_ms.saturating_sub(l.since_ms),
                expanded: false,
            });
        }
    }

    /// Forget every live buffer — the broker that was producing them is gone.
    pub(crate) fn drop_live(&mut self) {
        self.live.clear();
    }
}

impl ChatPane {
    /// One `Thought` fragment landed for `agent`.
    pub(crate) fn absorb_thought(&mut self, agent: String, text: String) {
        let now = crate::chattime::unix_now_ms();
        self.thoughts.absorb(stream_key(&agent), &text, now);
    }

    /// The run ended (or the broker went away): close every live block —
    /// tool lines and thoughts alike — as the one call `fold_swarm` makes.
    pub(crate) fn abandon_blocks(&mut self) {
        let now = crate::chattime::unix_now_ms();
        self.tools.abandon(now);
        self.thoughts.abandon(now);
    }

    /// Whether some agent's reasoning is still arriving — the redraw
    /// predicate (`panebusy::pane_animating`) for the live block's shimmer
    /// and clock. Bounded: every buffer ends in `settle` when the reply
    /// lands, or in `abandon` when the run does.
    pub(crate) fn thinking_live(&self) -> bool {
        !self.thoughts.live.is_empty()
    }
}

#[cfg(test)]
#[path = "chatthought_tests.rs"]
mod tests;
