//! The stdio broker loop behind the `/crew` pane. Reads `PluginCommand` JSON
//! lines and STREAMS every event as it happens (flushing per line). Long work
//! — a relay turn, /fan, /loop, /goal — spawns a **background task** (its own
//! worker thread, id, and cancel flag) so several run at once (up to
//! `CREW_MAX_TASKS`) while the main loop keeps draining stdin. Quick constructs
//! (/help, /model, …) answer inline; `/tasks` lists the running tasks and
//! `/stop [#n]` cancels all of them or just task #n between hops/rounds. Each
//! task's streamed `Message` events carry a `task:<id>` tag in their `meta`.
//! Used both by the `crew-broker-plugin` binary and by the `crew` binary
//! re-execing itself with `--broker-plugin`.
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::relay::{msg, multi_targets, relay_turn, split_target};
use super::session::{call_timeout, Session};
use crate::{PluginCommand, PluginEvent, Registry};

static THREAD_SEQ: AtomicU64 = AtomicU64::new(1);

/// stdout shared between the main loop and the worker thread; every emit
/// locks, writes one full line, and flushes.
type Out = Arc<Mutex<std::io::Stdout>>;

fn emit(out: &Out, ev: &PluginEvent) -> anyhow::Result<()> {
    // Auto-save the conversation as it streams (see sessionlog) — every
    // emitter funnels through here, worker threads included.
    if let PluginEvent::Message { sender, text, .. } = ev {
        super::sessionlog::append(sender, text);
    }
    let mut o = out.lock().unwrap_or_else(|e| e.into_inner());
    writeln!(o, "{}", serde_json::to_string(ev)?)?;
    o.flush()?;
    Ok(())
}

/// Run the broker over stdin/stdout until EOF.
pub fn run_broker_stdio() -> anyhow::Result<()> {
    // Before anything reads the env: import provider keys the launching app
    // didn't inherit (GUI / stale-terminal launches). Single-threaded here.
    super::shellenv::hydrate();
    // The previous run's conversation becomes resumable (/resume).
    super::sessionlog::rotate();
    let stdin = std::io::stdin();
    let out: Out = Arc::new(Mutex::new(std::io::stdout()));
    let mut session = Session::new();
    let mut tasks = super::tasks::Tasks::new();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(cmd) = serde_json::from_str::<PluginCommand>(&line) else {
            continue;
        };
        match cmd {
            PluginCommand::Hello { .. } => hello(&out, &session)?,
            PluginCommand::Send { text, .. } => send(text, &out, &mut session, &mut tasks)?,
            PluginCommand::Subscribe { .. } => {}
        }
    }
    // stdin closed (pane gone / EOF): let running tasks finish streaming
    // rather than truncating their output mid-line.
    tasks.join_all();
    Ok(())
}

fn hello(out: &Out, session: &Session) -> anyhow::Result<()> {
    let reg = session.registry();
    emit(
        out,
        &PluginEvent::Ready {
            v: 1,
            provider: "crew".into(),
            channels: vec!["crew".into()],
        },
    )?;
    emit(
        out,
        &PluginEvent::Roster {
            agents: reg.infos(),
        },
    )?;
    emit(out, &msg("crew", roster(&reg)))
}

/// Route one Send. `/stop [#N]`, `/tasks`, `/status`, and quick constructs
/// answer inline; every other task spawns a NEW background worker (up to the
/// cap), so several run at once.
fn send(
    text: String,
    out: &Out,
    session: &mut Session,
    tasks: &mut super::tasks::Tasks,
) -> anyhow::Result<()> {
    use std::sync::atomic::AtomicBool;
    use std::time::Instant;
    tasks.reap();
    let trimmed = text.trim().to_string();
    // Resolve built-in single-letter aliases (`/s` → `/status`) before ANY
    // routing below, so they reach the same interceptors their long form does.
    let trimmed = super::commands::expand_alias(&trimmed);
    // First whitespace token, so `/tasks` and `/status` tolerate trailing args
    // (they take none) instead of misrouting to "unknown construct".
    let cmd0 = trimmed.split_whitespace().next().unwrap_or("");

    // /stop [#N] — cancel one task or all.
    if trimmed == "/stop" || trimmed.starts_with("/stop ") {
        let arg = trimmed.strip_prefix("/stop").unwrap().trim();
        if arg.is_empty() {
            let n = tasks.cancel_all();
            let m = if n == 0 {
                "nothing is running".to_string()
            } else {
                format!("stopping all {n} task(s)\u{2026}")
            };
            return emit(out, &msg("crew", m));
        }
        let id: Option<u64> = arg.trim_start_matches('#').parse().ok();
        let m = match id {
            Some(id) if tasks.cancel(id) => format!("stopping task #{id}\u{2026}"),
            Some(id) => format!("no task #{id}"),
            None => "usage: /stop [#id]".to_string(),
        };
        return emit(out, &msg("crew", m));
    }

    // /tasks — list running tasks.
    if cmd0 == "/tasks" {
        let lines = tasks.describe(Instant::now());
        let body = if lines.is_empty() {
            "no background tasks running".to_string()
        } else {
            lines.join("\n")
        };
        return emit(out, &msg("crew", body));
    }

    // /status — session totals plus the LIVE task count (needs the registry,
    // so it's handled here rather than in commands::handle).
    if cmd0 == "/status" {
        return emit(
            out,
            &msg("crew", super::commands::status_report(session, tasks.len())),
        );
    }

    // `#note` — remember a standing preference (à la Claude Code's # memory):
    // appended to ./.crew/memory.md and prepended to every task from now on.
    // Answered inline; nothing dials an agent.
    if let Some(note) = trimmed.strip_prefix('#') {
        return emit(out, &msg("crew", super::memory::remember(note)));
    }

    if super::commands::is_quick(&trimmed) {
        return super::commands::handle(session, &trimmed, &mut |ev| emit(out, &ev));
    }

    if !tasks.admit() {
        return emit(
            out,
            &msg(
                "crew",
                format!(
                    "at capacity ({} tasks) \u{2014} /stop one first",
                    tasks.len()
                ),
            ),
        );
    }

    // The worker closure needs the task id (to stamp `meta` and print the
    // start/done lines), but `attach` needs the JoinHandle which only exists
    // after `spawn` — so reserve the id first, spawn, then attach.
    session.turns.fetch_add(1, Ordering::Relaxed);
    let label: String = trimmed.chars().take(40).collect();
    let cancel = std::sync::Arc::new(AtomicBool::new(false));
    let mut snap = session.snapshot_with_cancel(std::sync::Arc::clone(&cancel));
    let out_thread = Arc::clone(out);
    let is_cmd = super::commands::is_command(&trimmed);
    let id = tasks.reserve();
    // `tasks` doesn't count this task yet (it's attached below, once the
    // JoinHandle exists) — `+ 1` accounts for it so the line reflects live
    // capacity right as the task starts.
    let n = tasks.len() + 1;
    let max = super::tasks::Tasks::max();
    emit(
        out,
        &msg(
            "crew",
            format!("\u{25b8} task #{id} started \u{00b7} {n}/{max} \u{00b7} {label}"),
        ),
    )?;
    let handle = std::thread::spawn(move || {
        let tokens = Arc::clone(&snap.tokens);
        // Stamp every relay Message event with this task's id, and count Stats.
        let mut counting = |mut ev: PluginEvent| {
            if let PluginEvent::Stats { tokens: t, .. } = &ev {
                tokens.fetch_add(*t, Ordering::Relaxed);
            }
            if let PluginEvent::Message { meta, .. } = &mut ev {
                // Combine the task id with any existing `meta` (the hop latency,
                // e.g. "0.0s", which the app also renders as the log-line
                // latency) — an `if meta.is_empty()` guard would skip exactly
                // the agent replies the tag exists to disambiguate.
                *meta = if meta.is_empty() {
                    format!("task:{id}")
                } else {
                    format!("task:{id} \u{00b7} {meta}") // e.g. "task:3 · 0.0s"
                };
            }
            emit(&out_thread, &ev)
        };
        let res = if is_cmd {
            super::commands::handle(&mut snap, &trimmed, &mut counting)
        } else {
            relay_counting(&trimmed, &snap, &mut counting)
        };
        let done = match (res, snap.cancelled()) {
            (Err(e), _) => format!("\u{2717} task #{id}: {e}"),
            (Ok(_), true) => format!("\u{2717} task #{id} stopped"),
            (Ok(_), false) => format!("\u{2713} task #{id} done"),
        };
        let _ = emit(&out_thread, &msg("crew", done));
    });
    tasks.attach(id, label, cancel, handle, Instant::now());
    Ok(())
}

/// A human-readable description of which agents were discovered.
pub(crate) fn roster(reg: &Registry) -> String {
    if reg.is_empty() {
        return "No inbuilt agents available. Set OPENROUTER_API_KEY, \
                DASHSCOPE_API_KEY, or ANTHROPIC_API_KEY and reopen /crew."
            .into();
    }
    format!(
        "Detected {} agent(s): {}. Type a task and press Enter; prefix @<agent> \
         to choose who starts. Agents see the task + transcript and hand off with \
         a final `@next <agent>` line, or finish with `@done`.",
        reg.len(),
        reg.names().join(", "),
    )
}

fn relay_counting(
    input: &str,
    session: &Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("crew", roster(&reg)));
    }
    let task = input.trim();
    if task.is_empty() {
        return Ok(());
    }
    // `@a+b <task>` fans out to that subset in parallel instead of relaying.
    if let Some((names, body)) = multi_targets(task, &reg) {
        emit(msg(
            "crew",
            format!("fanning out to {} in parallel\u{2026}", names.join("+")),
        ))?;
        return super::fan::fan_out(&reg, &names, &body, call_timeout(), emit);
    }
    // A `/resume` before this task folds the previous session's tail in as
    // restored context (consumed once).
    let resumed = session
        .resume
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    let task_owned = match resumed {
        Some(prev) => super::sessionlog::with_resume(&prev, task),
        None => task.to_string(),
    };
    super::sessionlog::append("user", task);
    let (start, body) = split_target(&task_owned, &reg);
    let tid = format!("t{}", THREAD_SEQ.fetch_add(1, Ordering::Relaxed));
    emit(msg(
        "crew",
        format!("starting with {start} — relaying until an agent says @done"),
    ))?;
    let broker = session.broker(reg);
    relay_turn(&broker, &start, &body, &tid, emit).map(|_| ())
}

#[cfg(test)]
#[path = "stdio_tests.rs"]
mod tests;
