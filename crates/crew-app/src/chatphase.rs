//! What agent smith is doing, as the code line's word.
//!
//! The word used to be one of smith's ten at random, a turn: `jacking in`
//! while it read a file, `compiling` while it thought. It looked busy and said
//! nothing. Now it is the phase the turn is in, still in smith's voice and
//! still never a tool's name (the 0.26.104 rule): `tracing` while it reads or
//! searches, `rewriting` while it edits, `compiling` while a command runs,
//! `decrypting` in git's history, `jacking in` on the web, `replicating` while
//! a sub-agent works, `deciphering` while it thinks, `inevitable…` as the
//! answer comes.
//!
//! A word holds for [`DWELL_MS`] before the next one takes over. A burst of
//! reads and edits a few hundred ms apart would restart the decode on every
//! call, and the line would never show a word at all.
use crew_hive::HiveEvent;
use crew_plugin::PluginEvent;

/// How long a word holds before another phase may replace it.
pub(crate) const DWELL_MS: u64 = 1_500;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Phase {
    #[default]
    Routing,
    Thinking,
    Tracing,
    Rewriting,
    Compiling,
    Decrypting,
    Reaching,
    Replicating,
    Answering,
    Other,
}

impl Phase {
    pub(crate) fn word(self) -> &'static str {
        match self {
            Self::Routing => "rerouting",
            Self::Thinking => "deciphering",
            Self::Tracing => "tracing",
            Self::Rewriting => "rewriting",
            Self::Compiling => "compiling",
            Self::Decrypting => "decrypting",
            Self::Reaching => "jacking in",
            Self::Replicating => "replicating",
            Self::Answering => "inevitable\u{2026}",
            Self::Other => "assimilating",
        }
    }
}

/// The phase a call to the tool `label` puts the agent in: Claude Code's
/// names (`Read`, `Bash`, `mcp__x__y`) and crew's (`sys:read_file`).
pub(crate) fn of_tool(label: &str) -> Phase {
    if label.starts_with("lsp:") {
        return Phase::Tracing;
    }
    // The tool's own name: past a `server:` and an MCP `mcp__server__`.
    let tool = label.rsplit(':').next().unwrap_or(label);
    let tool = tool.rsplit("__").next().unwrap_or(tool);
    match tool.to_ascii_lowercase().as_str() {
        "read" | "read_file" | "grep" | "glob" | "ls" | "list_dir" | "outline" | "find_tools"
        | "notebookread" => Phase::Tracing,
        "edit" | "multiedit" | "write" | "write_file" | "notebookedit" => Phase::Rewriting,
        "run" | "bash" | "bashoutput" | "killshell" => Phase::Compiling,
        "git" => Phase::Decrypting,
        "fetch" | "search" | "webfetch" | "websearch" => Phase::Reaching,
        "task" | "agent" => Phase::Replicating,
        _ => Phase::Other,
    }
}

/// The phase an event says the turn is in, when it says one. A tool's
/// RESULT puts the agent back to thinking about what came back. The API
/// engine's calls arrive as `[tool]` cards: the call's names the tool, its
/// result's first line carries `✓` or `✗`.
pub(crate) fn of_event(ev: &PluginEvent) -> Option<Phase> {
    match ev {
        PluginEvent::Hive {
            event: HiveEvent::ToolCall { label, .. },
        } => Some(of_tool(label)),
        PluginEvent::Hive {
            event: HiveEvent::ToolResult { .. },
        } => Some(Phase::Thinking),
        PluginEvent::Thought { .. } => Some(Phase::Thinking),
        PluginEvent::Delta { sub: false, .. } => Some(Phase::Answering),
        PluginEvent::Message { text, .. } => {
            let card = text.strip_prefix(crate::chatcard::TOOL_PREFIX)?;
            let first = card.lines().next().unwrap_or_default();
            match first.contains(['\u{2713}', '\u{2717}']) {
                true => Some(Phase::Thinking),
                false => first.split_whitespace().next().map(of_tool),
            }
        }
        _ => None,
    }
}

/// The word on show, when it went up, and the phase waiting to replace it.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PhaseClock {
    shown: Phase,
    since: u64,
    next: Option<(Phase, u64)>,
}

impl PhaseClock {
    /// A turn that began at `now`: it is routing until something says more.
    pub(crate) fn start(now: u64) -> Self {
        Self {
            since: now,
            ..Self::default()
        }
    }

    /// Fold in an event heard at `now`.
    pub(crate) fn hear(&mut self, ev: &PluginEvent, now: u64) {
        let Some(p) = of_event(ev) else { return };
        (self.shown, self.since) = self.at(now);
        let waiting = self.next.filter(|n| n.0 == p).map_or(now, |n| n.1);
        self.next = (p != self.shown).then_some((p, waiting));
    }

    /// [`Self::at`] as the code line takes it: the word, and since when.
    pub(crate) fn word_at(&self, now: u64) -> (&'static str, u64) {
        let (p, since) = self.at(now);
        (p.word(), since)
    }

    /// The phase on show at `now`, and since when: a waiting phase takes
    /// over once the shown word has held for [`DWELL_MS`].
    pub(crate) fn at(&self, now: u64) -> (Phase, u64) {
        match self.next {
            Some((p, t)) if now >= t.max(self.since + DWELL_MS) => {
                (p, t.max(self.since + DWELL_MS))
            }
            _ => (self.shown, self.since),
        }
    }
}

#[cfg(test)]
#[path = "chatphase_tests.rs"]
mod tests;
