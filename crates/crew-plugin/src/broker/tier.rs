//! What a tool can DO TO THE WORLD, which is a different question from what it is for.
//!
//! An assistant that reaches everyday life — mail, calendars, money, someone's front door —
//! needs one place that answers "can this be undone?" before it fires. That answer cannot live
//! in each tool's own code, because the tools that matter most will be MCP servers written by
//! other people. So it lives here, as a classification the daemon's gate reads.
//!
//! The tiers are deliberately about REVERSIBILITY, not danger. "Delete a temp file" and "wire
//! money" are both irreversible; the gate asks about both and lets the human weigh them.
use crate::mcp::McpTool;

/// How far a tool's effects reach, and whether they can be taken back.
#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub enum Tier {
    /// Observes only. Nothing outside crew changes.
    Read,
    /// Changes something that can be put back the way it was, by us, without asking anyone.
    Reversible,
    /// Cannot be undone by us: it left the machine, told a person, spent money, or destroyed
    /// something. Every one of these must be approved before it fires.
    Irreversible,
}

impl Tier {
    /// Human word, for prompts and the ledger.
    pub fn label(self) -> &'static str {
        match self {
            Tier::Read => "read",
            Tier::Reversible => "reversible",
            Tier::Irreversible => "irreversible",
        }
    }

    /// Does firing this need a human to say yes first?
    pub fn needs_approval(self) -> bool {
        self == Tier::Irreversible
    }
}

/// The built-in `sys` surface, classified. Exhaustive by construction: [`sys_tier`] returns
/// `None` for anything not listed, and a test walks [`crate::broker::systools::tools`] and fails
/// on a `None` — so a fifth sys tool cannot ship unclassified.
///
/// `write_file` is REVERSIBLE rather than irreversible on purpose: it changes a file on the
/// user's own disk, which is recoverable in a way that sending a message to another human is
/// not. `run` is irreversible because a shell command is a blank cheque — `rm -rf`, `git push`,
/// `curl`. It is the one built-in that can do anything at all, so it is the one that asks.
pub fn sys_tier(tool: &str) -> Option<Tier> {
    Some(match tool {
        "read_file" | "list_dir" => Tier::Read,
        "write_file" => Tier::Reversible,
        "run" => Tier::Irreversible,
        _ => return None,
    })
}

/// The tier of any tool on the `@tool server:name` surface.
///
/// An MCP server crew has never seen before gets [`Tier::Irreversible`]: we do not know what it
/// does, and "unknown" must mean "ask" rather than "go ahead". The cost of that default is a
/// prompt; the cost of the other default is an assistant that mailed something on your behalf
/// because nobody had written a rule yet.
pub fn tier_of(server: &str, tool: &str) -> Tier {
    match server {
        "sys" => sys_tier(tool).unwrap_or(Tier::Irreversible),
        _ => Tier::Irreversible,
    }
}

/// Classify a whole tool descriptor.
pub fn tier_of_tool(t: &McpTool) -> Tier {
    tier_of(&t.server, &t.name)
}

/// Tools blocked by `CREW_SYS_MODE=readonly`: anything that is not [`Tier::Read`]. This used to
/// be a hand-kept `matches!(tool, "run" | "write_file")` beside the tier table — two lists of
/// mutating tools, one of which would eventually be updated without the other.
pub fn blocked_by_read_only(tool: &str) -> bool {
    sys_tier(tool).is_some_and(|t| t != Tier::Read)
}

#[cfg(test)]
#[path = "tier_tests.rs"]
mod tests;
