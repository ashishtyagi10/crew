//! Which agent runs a todo, and how it is told the task.
//!
//! Choosing between three CLIs and the smith pane is not a question that
//! needs a model: a call to decide it would cost money and a round trip
//! before any work starts, and its answer would be no better than a short
//! ladder whose every rung is a fact the user can check. So this is a pure
//! function over `(assignee, markers, PATH, sign-ins, pin)` returning the
//! agent AND the reason, which the pane's first line and the activity log
//! both print — a choice that cannot be read back is a choice that cannot
//! be corrected.
//!
//! Crew launches each CLI the way the user would at a prompt, with the
//! task as its opening prompt and NO skip-permissions flag: "autonomous"
//! means crew does not wait for you to type the task, not that the agent
//! skips its own permission model.
use std::path::Path;

/// The agents a todo can be handed to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Agent {
    Claude,
    Codex,
    Opencode,
    /// The agent smith pane: the fallback when no CLI is on PATH.
    Smith,
}

/// The CLI rungs in tie-break order.
const CLIS: [Agent; 3] = [Agent::Claude, Agent::Codex, Agent::Opencode];

/// Every agent, in the order the `#` popup offers them.
pub(crate) const AGENTS: [Agent; 4] = [Agent::Claude, Agent::Codex, Agent::Opencode, Agent::Smith];

impl Agent {
    /// The word the chip, the label and the reason use.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
            Agent::Opencode => "opencode",
            Agent::Smith => "smith",
        }
    }

    /// The binary on PATH; `None` for smith, which is crew itself.
    pub(crate) fn bin(self) -> Option<&'static str> {
        match self {
            Agent::Smith => None,
            a => Some(a.name()),
        }
    }

    /// An `#assignee` that names an agent. Any other name is a person,
    /// exactly as before — only these four words changed meaning, and only
    /// at run time.
    pub(crate) fn from_assignee(who: &str) -> Option<Agent> {
        let who = who.to_ascii_lowercase();
        AGENTS
            .into_iter()
            .find(|a| a.name() == who || (*a == Agent::Claude && who == "claude-code"))
    }

    /// The `/model` sign-in row that means this CLI.
    fn signin_name(self) -> &'static str {
        match self {
            Agent::Claude => "claude-code",
            a => a.name(),
        }
    }

    /// How the CLI takes an opening prompt, verified against the installed
    /// binaries: `claude [prompt]` and `codex [prompt]` are positional;
    /// opencode's TUI takes `--prompt`. Smith is not a process crew spawns.
    pub(crate) fn invocation(self, task: &str) -> Option<(&'static str, Vec<String>)> {
        Some(match self {
            Agent::Claude => ("claude", vec![task.to_string()]),
            Agent::Codex => ("codex", vec![task.to_string()]),
            Agent::Opencode => ("opencode", vec!["--prompt".to_string(), task.to_string()]),
            Agent::Smith => return None,
        })
    }
}

/// The agent-config files at a project root, each a vote for the tool that
/// reads it. `AGENTS.md` is read by codex AND opencode, so it only counts
/// when neither of the other two speaks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Markers {
    pub claude: bool,
    pub opencode: bool,
    pub agents_md: bool,
}

impl Markers {
    pub(crate) fn at(root: &Path) -> Markers {
        Markers {
            claude: root.join("CLAUDE.md").exists() || root.join(".claude").is_dir(),
            opencode: root.join("opencode.json").exists() || root.join(".opencode").is_dir(),
            agents_md: root.join("AGENTS.md").exists(),
        }
    }

    /// The one agent the markers vote for, and the file that says so. Two
    /// votes tie and fall through to the next rung.
    fn vote(self) -> Option<(Agent, &'static str)> {
        match (self.claude, self.opencode, self.agents_md) {
            (true, false, _) => Some((Agent::Claude, "CLAUDE.md")),
            (false, true, _) => Some((Agent::Opencode, "opencode.json")),
            (false, false, true) => Some((Agent::Codex, "AGENTS.md")),
            _ => None,
        }
    }
}

/// What the machine knows: which CLIs are on PATH, which the `/model`
/// picker's rows say are signed in, and which provider serves.
pub(crate) struct Facts<'a> {
    pub on_path: &'a [Agent],
    pub signed_in: &'a [&'a str],
    pub serving: Option<&'a str>,
}

impl Facts<'_> {
    fn signed(&self, a: Agent) -> bool {
        self.signed_in
            .iter()
            .any(|n| n.eq_ignore_ascii_case(a.signin_name()))
    }

    fn serves(&self, a: Agent) -> bool {
        self.serving
            .is_some_and(|s| s.eq_ignore_ascii_case(a.signin_name()))
    }
}

/// The ladder. `Err` carries the one-line reason nothing can run: an
/// assignee's CLI that is not installed is a refusal, not a fall-through —
/// the user named a tool and crew must not quietly hand the task to another.
pub(crate) fn pick(
    who: Option<&str>,
    markers: Markers,
    f: &Facts,
) -> Result<(Agent, String), String> {
    let on = |a: Agent| a == Agent::Smith || f.on_path.contains(&a);
    if let Some(a) = who.and_then(Agent::from_assignee) {
        return if on(a) {
            Ok((a, format!("#{}", a.name())))
        } else {
            Err(format!("{} is not on PATH", a.name()))
        };
    }
    if let Some((a, file)) = markers.vote().filter(|(a, _)| on(*a)) {
        let state = if f.signed(a) { ", signed in" } else { "" };
        return Ok((a, format!("{file}{state}")));
    }
    let mut ranked: Vec<Agent> = CLIS.iter().copied().filter(|a| on(*a)).collect();
    // Stable, so the CLIS order breaks every remaining tie.
    ranked.sort_by_key(|a| (!f.signed(*a), !f.serves(*a)));
    match ranked.first() {
        Some(&a) if f.signed(a) => Ok((
            a,
            if f.serves(a) {
                "signed in, serving".into()
            } else {
                "signed in".into()
            },
        )),
        Some(&a) => Ok((a, "on PATH".into())),
        None => Ok((Agent::Smith, "no CLI on PATH".into())),
    }
}

#[cfg(test)]
#[path = "agentpick_tests.rs"]
mod tests;
