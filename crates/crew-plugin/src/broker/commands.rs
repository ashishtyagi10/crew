//! Slash constructs typed into the `/crew` pane. Anything starting with `/`
//! is a broker command rather than a task: `/help` lists the constructs,
//! `/model` reports the roster with each agent's current model.
use crate::PluginEvent;

use super::adapter::Adapter;
use super::relay::msg;
use super::session::Session;

/// Whether `text` addresses the broker's command router.
pub(crate) fn is_command(text: &str) -> bool {
    text.trim_start().starts_with('/')
}

/// Whether a command answers instantly (no agent calls) — those run inline on
/// the stdin loop even while a long construct occupies the worker thread.
pub(crate) fn is_quick(text: &str) -> bool {
    let line = text.trim().trim_start_matches('/');
    let cmd = line.split_whitespace().next().unwrap_or("");
    // Retired commands (`/fan`, `/goal`, `/skill`, …) are absent on purpose:
    // they answer with an instant hint, so they must not occupy a worker slot.
    // `/restore` is the one construct left that touches files.
    is_command(text) && cmd != "restore"
}

/// One-line summaries of every construct, shown by `/help`.
pub(crate) const HELP: &str = "constructs:\n\
    /help — this list\n\
    /model — the roster with each agent's model (also in the pane footer)\n\
    /model <agent> <model|default> — pin an agent to a model (mix models freely)\n\
    /model all <model|default> — set every agent's model at once\n\
    plain language routes itself — \u{201c}have every agent take a crack at \u{2026}\u{201d} \
    fans out in parallel; \u{201c}keep refining \u{2026}\u{201d} runs improvement rounds; \
    \u{201c}keep working until \u{2026}\u{201d} loops with a judge until it rules the goal met; \
    \u{201c}draft a plan for \u{2026}\u{201d} waits for your \u{201c}approve\u{201d} or \
    \u{201c}reject\u{201d} before anything runs; \
    \u{201c}commit this\u{201d} drafts a commit message (say \u{201c}apply\u{201d} to create it); \
    \u{201c}look over my changes\u{201d}, \u{201c}what did I ship this week\u{201d} and \
    \u{201c}pick up where we left off\u{201d} reach code review, a standup and the last session\n\
    /restore [n] — list the automatic snapshots, or put snapshot n's files back\n\
    /diff — everything different from the last commit, new files included\n\
    /doctor — health-check the AI stack (provider, CLIs, MCP servers and tools, memory, session)\n\
    #<note> — remember a preference (ask \u{201c}what do you remember?\u{201d} to see them)\n\
    skills: drop .md playbooks into .crew/skills — a task that names one applies it by itself\n\
    /reload — re-read skills, plugin agents, and mcp.json without a restart\n\
    /stop [#n] — cancel all background tasks, or just task #n\n\
    @<agent> <task> — choose who starts the relay\n\
    @<a>+<b> <task> — those agents answer in parallel\n\
    \u{2026} tip: tasks run in the background — the footer lists them, /stop #n cancels one\n\
    aliases: /h /d /m /r\n\
    ";

/// The one-line summary `/help` gives for `name` (no leading slash), if it has
/// one: the text after the em dash on the first `HELP` line naming it.
///
/// Exposed so a host's palette shows the broker's own words rather than a
/// fourth hand-written copy of them. Membership of the construct lists is
/// pinned in every direction by tests; their DESCRIPTIONS were not, and drifted
/// — the palette told users `/goal` "set the crew's shared goal" while the
/// broker looped until a judge ruled it met. Neither the palette's sentence nor
/// any other list was wrong about which commands exist; it was wrong about what
/// one of them does, which no membership test can catch.
pub fn construct_summary(name: &str) -> Option<&'static str> {
    HELP.lines().find_map(|line| {
        let line = line.trim_start();
        let rest = line.strip_prefix('/')?;
        let end = rest.chars().take_while(char::is_ascii_alphanumeric).count();
        (rest.get(..end)? == name)
            .then(|| line.split_once(" \u{2014} ").map(|(_, s)| s.trim()))
            .flatten()
    })
}

/// Expand a built-in single-letter slash alias in the FIRST token, preserving
/// the rest: `/s` → `/status`, `/m coder qwen` → `/model coder qwen`. Returns
/// the input unchanged when the first token isn't a known alias.
pub fn expand_alias(trimmed: &str) -> String {
    const ALIASES: &[(&str, &str)] = &[
        ("/h", "/help"),
        ("/d", "/diff"),
        ("/m", "/model"),
        ("/r", "/reload"),
    ];
    let (head, rest) = trimmed
        .split_once(char::is_whitespace)
        .unwrap_or((trimmed, ""));
    for (short, long) in ALIASES {
        if head == *short {
            return if rest.is_empty() {
                (*long).to_string()
            } else {
                format!("{long} {rest}")
            };
        }
    }
    trimmed.to_string()
}

/// Construct names this router recognizes — the candidate list for
/// [`closest_construct`], and the source a host should build its palette
/// from rather than keeping a second copy (see [`constructs`]).
const CONSTRUCTS: &[&str] = &[
    "help", "model", "doctor", "restore", "reload", "diff", "stop",
];

/// Every construct the broker answers, without the leading slash. Exposed so
/// a host can ASSERT its palette against the router instead of maintaining a
/// parallel list — `/reload` shipped for eleven releases as a working command
/// no palette offered, because two lists existed and nothing compared them.
pub fn broker_constructs() -> &'static [&'static str] {
    CONSTRUCTS
}

/// The known construct whose name is closest to `typo`, if one is close
/// enough — sharing a 2+ char prefix, or within edit distance 2 — else `None`.
fn closest_construct(typo: &str) -> Option<&'static str> {
    let typo = typo.to_ascii_lowercase();
    CONSTRUCTS
        .iter()
        .filter(|c| {
            let shared_prefix = typo
                .chars()
                .zip(c.chars())
                .take_while(|(a, b)| a == b)
                .count();
            shared_prefix >= 2 || levenshtein(&typo, c) <= 2
        })
        .min_by_key(|c| levenshtein(&typo, c))
        .copied()
}

/// Classic edit-distance DP (insert/delete/substitute, unit cost).
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (la, lb) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; lb + 1]; la + 1];
    for (i, row) in dp.iter_mut().enumerate().take(la + 1) {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate().take(lb + 1) {
        *cell = j;
    }
    for i in 1..=la {
        for j in 1..=lb {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[la][lb]
}

/// Handle a `/command` line; emits reply events through `emit`. `_tick_emit`
/// stays in the signature for symmetry with the intent router's dispatch —
/// no surviving construct dials an agent mid-hop any more (the last that did,
/// `/goal` and `/approve`, retired behind the router).
pub(crate) fn handle(
    session: &mut Session,
    text: &str,
    _tick_emit: &std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let line = text.trim().trim_start_matches('/');
    let (cmd, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    // Retired as commands: the intent router recognizes the plain-language
    // ask and reaches the same capability. The hint teaches the phrasing
    // rather than silently reinterpreting.
    if let Some((_, hint)) = super::retired::RETIRED.iter().find(|(c, _)| *c == cmd) {
        return emit(msg("agent smith", *hint));
    }
    match cmd {
        "help" => emit(msg("agent smith", HELP)),
        "model" => super::modelcmd::model_cmd(session, rest, emit),
        "restore" => super::checkpoint::restore_cmd(rest, emit),
        "diff" => super::diff::diff_cmd(emit),
        "doctor" => emit(msg(
            "agent smith",
            super::doctor::render(&super::doctor::gather(session)),
        )),
        "reload" => reload_cmd(session, emit),
        other => emit(msg(
            "agent smith",
            match closest_construct(other) {
                Some(s) => format!("unknown construct /{other} — did you mean /{s}? (or /help)"),
                None => format!("unknown construct /{other} — try /help"),
            },
        )),
    }
}

/// `/reload` — pick up extension edits without a restart. Skills and plugin
/// manifests already re-read from disk on every use, so this reports what's
/// there now; MCP is forced to re-read `mcp.json`, drop every connection, and
/// re-list tools on next use. A fresh Roster event updates the pane's badges.
fn reload_cmd(
    session: &mut Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let skills = super::skills::load();
    let plugins: Vec<String> = super::plugins::load()
        .iter()
        .filter(|p| p.probe())
        .map(|p| p.name().to_string())
        .collect();
    let servers = session.lock_mcp().reload();
    emit(PluginEvent::Roster {
        agents: session.registry().infos(),
    })?;
    let list = |names: Vec<String>| {
        if names.is_empty() {
            "none".to_string()
        } else {
            format!("{} \u{2014} {}", names.len(), names.join(", "))
        }
    };
    emit(msg(
        "agent smith",
        format!(
            "reloaded from disk:\n\
             \u{25aa} skills: {}\n\
             \u{25aa} plugin agents: {}\n\
             \u{25aa} mcp: {}",
            list(skills.into_iter().map(|s| s.name).collect()),
            list(plugins),
            if servers.is_empty() {
                "none configured".to_string()
            } else {
                format!("{} \u{2014} reconnecting on next use", servers.join(", "))
            },
        ),
    ))
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "commands_retire_tests.rs"]
mod retire_tests;
