//! Slash constructs typed into the `/crew` pane. Anything starting with `/`
//! is a broker command rather than a task: `/help` lists the constructs,
//! `/agents` reports the roster with each agent's current model.
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
    is_command(text)
        && !matches!(
            cmd,
            "commit"
                | "review"
                | "standup"
                | "fan"
                | "loop"
                | "goal"
                | "skill"
                | "mcp"
                | "plan"
                | "approve"
                | "checkpoint"
                | "restore"
        )
}

/// One-line summaries of every construct, shown by `/help`.
pub(crate) const HELP: &str = "constructs:\n\
    /help — this list\n\
    /agents — the roster with each agent's model\n\
    /model <agent> <model|default> — pin an agent to a model (mix models freely)\n\
    /fan <task> — every agent answers the same task in parallel\n\
    /loop <n> <task> — n relay rounds, each improving the last answer\n\
    /goal <text> — keep working until a judge agent rules the goal met\n\
    /plan <task> — draft a numbered plan; nothing runs until /approve\n\
    /approve — execute the pending plan\n\
    /reject — discard the pending plan\n\
    /checkpoint [label] — snapshot the working tree (HEAD and index untouched)\n\
    /checkpoints — list saved snapshots\n\
    /restore <n> — put checkpoint n's files back\n\
    /diff — show the working tree's changes (git diff --stat)\n\
    /commit — draft an AI commit message · /commit apply — create the commit\n\
    /review — AI code review of the working diff, findings worst-first\n\
    /resume — fold the previous session's tail into the next task\n\
    /doctor — health-check the AI stack (provider, CLIs, MCP, memory)\n\
    /standup [days] — an AI standup update from recent commits\n\
    /cwd — show the working directory and sandbox mode\n\
    /skills — list prompt playbooks (~/.config/crew/skills, .crew/skills)\n\
    /skill <name> <task> — run the relay with that playbook prepended\n\
    /memory — show the standing memory prepended to every task\n\
    #<note> — remember a preference in ./.crew/memory.md\n\
    /mcp — MCP servers and their tools (~/.config/crew/mcp.json, .crew/mcp.json)\n\
    /reload — re-read skills, plugin agents, and mcp.json without a restart\n\
    /tasks — list the background tasks running now\n\
    /stop [#n] — cancel all background tasks, or just task #n\n\
    /status — session totals, models, and the live task count\n\
    @<agent> <task> — choose who starts the relay\n\
    @<a>+<b> <task> — those agents answer in parallel\n\
    \u{2026} tip: tasks run in the background — /tasks lists them, /stop #n cancels one\n\
    aliases: /h /a /s /t /d /m /r\n\
    ";

/// Expand a built-in single-letter slash alias in the FIRST token, preserving
/// the rest: `/s` → `/status`, `/m coder qwen` → `/model coder qwen`. Returns
/// the input unchanged when the first token isn't a known alias.
pub(crate) fn expand_alias(trimmed: &str) -> String {
    const ALIASES: &[(&str, &str)] = &[
        ("/h", "/help"),
        ("/a", "/agents"),
        ("/s", "/status"),
        ("/t", "/tasks"),
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

/// Construct names this router (and the stdio layer's `/stop`/`/tasks`/
/// `/status`) recognizes — the candidate list for [`closest_construct`].
const CONSTRUCTS: &[&str] = &[
    "help",
    "agents",
    "model",
    "fan",
    "loop",
    "goal",
    "plan",
    "approve",
    "reject",
    "checkpoint",
    "checkpoints",
    "commit",
    "review",
    "resume",
    "doctor",
    "standup",
    "restore",
    "skills",
    "skill",
    "memory",
    "mcp",
    "reload",
    "diff",
    "cwd",
    "status",
    "tasks",
    "stop",
];

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

/// Handle a `/command` line; emits reply events through `emit`.
pub(crate) fn handle(
    session: &mut Session,
    text: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let line = text.trim().trim_start_matches('/');
    let (cmd, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    match cmd {
        "help" => emit(msg("crew", HELP)),
        "agents" => {
            // Fresh Roster first so the pane's badges pick up manifest edits.
            emit(PluginEvent::Roster {
                agents: session.registry().infos(),
            })?;
            emit(msg("crew", agents_report(session)))
        }
        "model" => model_cmd(session, rest, emit),
        "fan" => fan_cmd(session, rest, emit),
        "loop" => super::constructs::loop_cmd(session, rest, emit),
        "goal" => super::constructs::goal_cmd(session, rest, emit),
        "plan" => super::plan::plan_cmd(session, rest, emit),
        "approve" => super::plan::approve_cmd(session, emit),
        "reject" => super::plan::reject_cmd(session, emit),
        "checkpoint" => super::checkpoint::checkpoint_cmd(rest, emit),
        "checkpoints" => super::checkpoint::list_cmd(emit),
        "restore" => super::checkpoint::restore_cmd(rest, emit),
        "diff" => super::diff::diff_cmd(emit),
        "commit" => super::gitmsg::commit_cmd(session, rest, emit),
        "review" => super::review::review_cmd(session, emit),
        "doctor" => emit(msg("crew", super::doctor::render(&super::doctor::gather()))),
        "standup" => super::standup::standup_cmd(session, rest, emit),
        "resume" => {
            let m = match super::sessionlog::tail() {
                Some(prev) => {
                    let n = prev.len();
                    *session.resume.lock().unwrap_or_else(|e| e.into_inner()) = Some(prev);
                    format!(
                        "restored {n} chars of the previous session — the next \
                         task carries it as context"
                    )
                }
                None => "nothing to resume — no previous session found".into(),
            };
            emit(msg("crew", m))
        }
        "cwd" => cwd_cmd(emit),
        "skills" => emit(msg(
            "crew",
            super::skillframe::list_report(&super::skills::load()),
        )),
        "skill" => super::skills::skill_cmd(session, rest, emit),
        "memory" => emit(msg("crew", super::memory::report())),
        "mcp" => {
            let report = session.lock_mcp().report();
            emit(msg("crew", report))
        }
        "reload" => reload_cmd(session, emit),
        other => emit(msg(
            "crew",
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
        "crew",
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

/// `/cwd` — where the sys tools operate, and the sandbox mode.
fn cwd_report(dir: &std::path::Path, read_only: bool) -> String {
    let mode = if read_only { "read-only" } else { "full" };
    format!("working dir: {}\nsys: {mode}", dir.display())
}

/// `/cwd` — reports the sys tools' working directory and sandbox mode
/// (codex-style: read-only, no agent calls).
fn cwd_cmd(emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>) -> anyhow::Result<()> {
    let report = match std::env::current_dir() {
        Ok(dir) => cwd_report(&dir, super::systools::read_only()),
        Err(e) => format!("cwd unavailable: {e}"),
    };
    emit(msg("crew", report))
}

/// `/model` — list each agent's model; `/model <agent> <model>` — pin the
/// agent to that model for this session; `default` clears the pin. Re-emits
/// the roster so the pane's model badges update live.
fn model_cmd(
    session: &mut Session,
    rest: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut parts = rest.split_whitespace();
    let (agent, model) = (parts.next(), parts.next());
    let Some(agent) = agent else {
        return emit(msg("crew", agents_report(session)));
    };
    let reg = session.registry();
    let Some(name) = reg
        .names()
        .into_iter()
        .find(|n| n.eq_ignore_ascii_case(agent))
    else {
        return emit(msg(
            "crew",
            format!(
                "unknown agent \u{201c}{agent}\u{201d} — agents: {}",
                reg.names().join(", ")
            ),
        ));
    };
    let Some(model) = model else {
        return emit(msg("crew", format!("usage: /model {name} <model|default>")));
    };
    let note = if model.eq_ignore_ascii_case("default") {
        session.overrides.remove(&name);
        format!("{name} back on the provider default model")
    } else {
        session.overrides.insert(name.clone(), model.to_string());
        format!("{name} now runs {model}")
    };
    emit(PluginEvent::Roster {
        agents: session.registry().infos(),
    })?;
    emit(msg("crew", note))
}

/// `/fan <task>` — every agent answers `task` concurrently.
fn fan_cmd(
    session: &mut Session,
    task: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let task = task.trim();
    if task.is_empty() {
        return emit(msg("crew", "usage: /fan <task>"));
    }
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("crew", super::stdio::roster(&reg)));
    }
    let names = reg.names();
    emit(msg(
        "crew",
        format!("fanning out to {} agents in parallel\u{2026}", names.len()),
    ))?;
    super::fan::fan_out(&reg, &names, task, super::session::call_timeout(), emit)
}

/// `/status` — what the session has done and is doing right now. `tasks_running`
/// is the broker's live background-task count (0 = idle).
pub(crate) fn status_report(session: &Session, tasks_running: usize) -> String {
    use std::sync::atomic::Ordering;
    let running = if tasks_running == 0 {
        "idle".to_string()
    } else {
        format!(
            "{tasks_running} task{} running",
            if tasks_running == 1 { "" } else { "s" }
        )
    };
    let pins = if session.overrides.is_empty() {
        "none".to_string()
    } else {
        let mut pins: Vec<String> = session
            .overrides
            .iter()
            .map(|(a, m)| format!("{a} \u{2192} {m}"))
            .collect();
        pins.sort();
        pins.join(", ")
    };
    let turns = session.turns.load(Ordering::Relaxed);
    let tokens = session.tokens.load(Ordering::Relaxed);
    // "approx" until there's at least one turn to average over — a `0/turn`
    // reading would be meaningless (and turns == 0 would divide by zero).
    let approx = match tokens.checked_div(turns) {
        Some(avg) => format!("~{avg}/turn"),
        None => "approx".to_string(),
    };
    format!(
        "status: {running}\n\
         turns: {turns} \u{00b7} ~{tokens} tok ({approx})\n\
         model pins: {pins}\n\
         sys: {}\n\
         budget: {}\n\n{}",
        super::systools::mode_label(),
        budget_label(super::session::token_budget()),
        agents_report(session),
    )
}

/// The relay token budget as a human label: `"unlimited"` when unset (0,
/// `CREW_BROKER_TOKEN_BUDGET`), else `"~<n> tok"`.
fn budget_label(budget: usize) -> String {
    if budget == 0 {
        "unlimited".to_string()
    } else {
        format!("~{budget} tok")
    }
}

/// The roster, one agent per line: name, role hint, and the model it runs.
fn agents_report(session: &Session) -> String {
    let reg = session.registry();
    if reg.is_empty() {
        return super::stdio::roster(&reg);
    }
    let lines: Vec<String> = reg
        .infos()
        .iter()
        .map(|a| {
            let model = if a.model.is_empty() {
                "(own model)".to_string()
            } else {
                a.model.clone()
            };
            format!("\u{25aa} {} \u{2014} {} \u{2014} {model}", a.name, a.role)
        })
        .collect();
    lines.join("\n")
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
