//! The broker itself: the agents it routes between, the limits a thread runs
//! under, and what a session attaches to it.
use std::time::Duration;

use crate::broker::Registry;

/// Routes messages between agents in a [`Registry`], with a per-call timeout, a
/// maximum hop count, and an approximate token budget (0 = unlimited).
pub struct Broker {
    pub registry: Registry,
    pub max_hops: u32,
    pub timeout: Duration,
    pub token_budget: usize,
    /// Checked between hops; when it flips true (`/stop`), the thread ends
    /// with a Terminated hop instead of dialling the next agent.
    pub(crate) cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// When set, agents can call tools mid-relay (see [`crate::broker::toolcall`]).
    pub(crate) tools: Option<std::sync::Arc<dyn crate::broker::toolcall::ToolRunner>>,
    /// When set, transcript overflow is summarized instead of dropped
    /// (see [`crate::broker::compact`]); `None` keeps the fixed-tail clipping.
    pub(super) summarizer: Option<crate::broker::compact::Summarize>,
    /// Tool rounds one hop may take ([`MAX_TOOL_ROUNDS`] unless raised).
    pub(crate) tool_rounds: u32,
}

impl Broker {
    pub fn new(registry: Registry, max_hops: u32, timeout: Duration) -> Self {
        Self {
            registry,
            max_hops,
            timeout,
            token_budget: 0,
            cancel: None,
            tools: None,
            summarizer: None,
            tool_rounds: crate::broker::toolcall::MAX_TOOL_ROUNDS,
        }
    }

    /// Raise (or lower) the tool rounds one hop may take.
    pub(crate) fn with_tool_rounds(mut self, n: u32) -> Self {
        self.tool_rounds = n.max(1);
        self
    }

    /// Attach a transcript summarizer (`None` = keep clipping).
    pub(crate) fn with_summarizer(mut self, s: Option<crate::broker::compact::Summarize>) -> Self {
        self.summarizer = s;
        self
    }

    /// Cap a thread's approximate token spend (0 = unlimited).
    pub fn with_budget(mut self, tokens: usize) -> Self {
        self.token_budget = tokens;
        self
    }

    /// Attach a cooperative cancel flag (the `/stop` construct).
    pub fn with_cancel_flag(mut self, flag: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Self {
        self.cancel = Some(flag);
        self
    }

    pub(in crate::broker) fn cancelled(&self) -> bool {
        self.cancel
            .as_ref()
            .is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed))
    }
}
