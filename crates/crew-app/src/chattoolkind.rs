//! What KIND of row a tool block holds (see [`crate::chattool`]). A skill
//! spliced into the prompt, an MCP server that answered its handshake, a
//! language server that started — none is a tool call (nothing ran, nothing
//! to time or approve), yet each shapes the reply the block sits above, and
//! until these lines a skill rewrote the prompt in silence and a server's
//! cold start read as a hung call. So they are lines of the same block:
//! born done, marked by what they are rather than how they went, counted in
//! the summary by kind. The broker's `HiveEvent::Loaded` is what lands here.
use crate::chattool::{ToolBlock, ToolDone, ToolLine, ToolLines};
use crate::glyphs::Glyph;

/// The name a load with no agent of its own is filed under when no agent's
/// block is open — the swarm plan line's own sender, which the block then
/// anchors above (or the next reply's, see [`ToolLines::settle_loaded`]).
pub(crate) const LEAD: &str = "agent smith";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LineKind {
    /// A tool call: the spinner, then `✓`/`✗` and its duration.
    Tool,
    Skill,
    Mcp,
    Lsp,
}

impl LineKind {
    /// The wire `kind` of a `Loaded` event. `None` for one this build does
    /// not know — dropped rather than mislabelled, the way an unknown event
    /// tag is.
    pub(crate) fn parse(kind: &str) -> Option<Self> {
        match kind {
            "skill" => Some(Self::Skill),
            "mcp" => Some(Self::Mcp),
            "lsp" => Some(Self::Lsp),
            _ => None,
        }
    }

    /// The mark a load line carries in place of the outcome mark.
    pub(crate) fn glyph(self) -> Glyph<'static> {
        match self {
            Self::Tool => Glyph::Tool,
            Self::Skill => Glyph::Skill,
            Self::Mcp => Glyph::Mcp,
            Self::Lsp => Glyph::Lsp,
        }
    }

    /// The word the line and the LOG lead with: `skill rust-testing`.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Skill => "skill",
            Self::Mcp => "mcp",
            Self::Lsp => "lsp",
        }
    }

    /// The summary's count noun, singular.
    fn noun(self) -> &'static str {
        match self {
            Self::Tool => "tool call",
            Self::Skill => "skill",
            Self::Mcp => "mcp server",
            Self::Lsp => "language server",
        }
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// The summary line's text after its mark: `4 tool calls · 1 skill · 2.1 s`.
/// Loads are counted by kind, zero counts omitted; the time is the calls'
/// alone and goes with them (a block of loads has nothing to time). An
/// empty block still reads `0 tool calls · 0 ms`.
pub(crate) fn summary_text(b: &ToolBlock) -> String {
    let (calls, _, ms) = b.tally();
    let with_calls = calls > 0 || b.lines.is_empty();
    let mut parts = Vec::new();
    if with_calls {
        parts.push(format!(
            "{calls} {}{}",
            LineKind::Tool.noun(),
            plural(calls)
        ));
    }
    for k in [LineKind::Skill, LineKind::Mcp, LineKind::Lsp] {
        let n = b.lines.iter().filter(|l| l.kind == k).count();
        if n > 0 {
            parts.push(format!("{n} {}{}", k.noun(), plural(n)));
        }
    }
    if with_calls {
        parts.push(crate::chattoolline::fmt_ms(ms));
    }
    parts.join(" \u{b7} ")
}

impl ToolLines {
    /// A `Loaded` event becomes a line that is already done — no spinner,
    /// no duration; nothing ran. It goes under the agent it names; a load
    /// naming nobody (an MCP connect under whoever's call forced it) joins
    /// the open block of the agent the last hive event was about, else a
    /// block under [`LEAD`] that [`Self::settle_loaded`] hands to the next
    /// reply.
    pub(crate) fn absorb_loaded(&mut self, agent: &str, kind: &str, name: &str, detail: &str) {
        let Some(kind) = LineKind::parse(kind) else {
            return;
        };
        let id = match agent.is_empty() {
            false => self.id_for(agent),
            true => self
                .last_agent
                .filter(|id| self.blocks.iter().any(|b| b.agent_id == *id && !b.settled))
                .unwrap_or_else(|| self.id_for(LEAD)),
        };
        self.open_block(id).lines.push(ToolLine {
            kind,
            label: format!("{} {name}", kind.label()),
            args: String::new(),
            args_short: String::new(),
            started_ms: 0,
            done: Some(ToolDone {
                ok: true,
                ms: 0,
                text: detail.to_string(),
            }),
            show_text: false,
        });
    }

    /// The id `name`'s lines file under: the hive id already bound to it,
    /// else one minted from the name — top bit set, so it can never meet a
    /// hive id (those count up from zero) — and bound for next time.
    fn id_for(&mut self, name: &str) -> u64 {
        if let Some((id, _)) = self.names.iter().find(|(_, n)| n.as_str() == name) {
            return *id;
        }
        let id = name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
        }) | (1 << 63);
        self.names.insert(id, name.to_string());
        id
    }

    /// `agent` replied with no open block of its own: the oldest open block
    /// made of loads alone becomes its, anchored above that reply — a skill
    /// applied before anyone was named heads whichever reply it shaped.
    pub(crate) fn settle_loaded(&mut self, agent: &str, ts: &str) {
        let loads_only = |b: &&mut ToolBlock| {
            !b.settled && !b.lines.is_empty() && b.lines.iter().all(|l| l.kind != LineKind::Tool)
        };
        if let Some(b) = self.blocks.iter_mut().find(loads_only) {
            b.agent = agent.to_string();
            b.anchor = Some(ts.to_string());
            b.settled = true;
            b.expanded = false;
        }
    }
}

#[cfg(test)]
#[path = "chattoolkind_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chattoolanchor_tests.rs"]
mod anchor_tests;
