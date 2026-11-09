//! What one hop's reply used and cost, as the pane's stats and context bar
//! read it.

/// Real token usage from one API reply, as the provider reported it.
/// `input_tokens` is the call's full prompt size — the agent's live context
/// fill. All zeros when the backend can't report usage (external CLIs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// Micro-USD for this reply: the provider's exact figure when reported,
    /// else the pricing-table estimate for the model that answered; 0 =
    /// unknown.
    pub cost_microusd: u64,
    /// `qwen-flash · qwen-max: <why>` — the model that answered, then what
    /// its chain skipped first — when the provider's model chain fell
    /// through; empty when the first model answered, or the backend has no
    /// chain. Agent smith says it once per agent a turn (`fellback`).
    pub fell_back: String,
    /// The answering model's context window in tokens, when the backend says
    /// it (Claude Code's `result` line); 0 = look it up by model name.
    pub context_window: u64,
    /// `! not allowed: Write notes.txt …` when Claude Code's settings
    /// refused tool calls on this hop (`clauderesult::note`): smith's row
    /// under the reply (`fellback::Told::line`), never the reply's own text.
    pub refused: String,
    /// `! not run: sys:read_file README.md — the turn ended first` when the
    /// thread's final answer still held calls (`crew_hive::tools::not_run`):
    /// lifted out of the answer and said as smith's row under it, like
    /// `refused`, so the raw `@tool` line is not drawn as the agent's words.
    pub not_run: String,
}

impl Usage {
    /// What reply `c` to a request for `requested` used and cost: the
    /// provider's exact cost when it said one, else the pricing table's for
    /// the model that ANSWERED (`pricing::estimate` reads `c.model` before
    /// `requested`), which after a fallback is not the one asked for.
    pub(crate) fn of(requested: &str, c: &crew_hive::Completion) -> Self {
        Self {
            input_tokens: c.input_tokens,
            output_tokens: c.output_tokens,
            cost_microusd: match c.cost_microusd {
                0 => crew_hive::pricing::estimate(requested, c).unwrap_or(0),
                exact => exact,
            },
            fell_back: crate::broker::fellback::detail(c),
            context_window: 0,
            refused: String::new(),
            not_run: String::new(),
        }
    }

    /// `next` as the hop's usage — its prompt is the latest context fill —
    /// still carrying a fallback or a refusal an earlier dial of the same
    /// hop made: the chain fell through then, and the write was refused
    /// then, whatever the dial after it did.
    pub(crate) fn latest(&mut self, next: Usage) {
        let earlier = std::mem::take(&mut self.fell_back);
        let refused = std::mem::take(&mut self.refused);
        *self = next;
        if self.fell_back.is_empty() {
            self.fell_back = earlier;
        }
        if self.refused.is_empty() {
            self.refused = refused;
        }
    }
}
