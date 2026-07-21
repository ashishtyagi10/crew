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

use super::relay::{dialed_target, msg, multi_targets, relay_turn, split_target};
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
    emit(out, &msg("agent smith", startup_banner(&reg)))
}

/// The Agent Smith opening splash — a boxed nameplate over a binary "code
/// rain" tagline, in the spirit of the opencode/claude/codex startup banners.
/// Rendered in the theme ink (Matrix green on the CRT themes). Kept
/// ASCII-narrow so it survives slim panes; the chat renderer preserves its
/// line breaks (soft breaks are hard breaks in chat), so the box stays intact.
fn nameplate_art() -> String {
    let plate = "A G E N T   S M I T H";
    let pad = 3;
    let bar = "\u{2550}".repeat(plate.chars().count() + pad * 2);
    let sp = " ".repeat(pad);
    format!(
        "\u{2554}{bar}\u{2557}\n\
         \u{2551}{sp}{plate}{sp}\u{2551}\n\
         \u{255a}{bar}\u{255d}\n\
         01001101 \u{22ee} the sound of inevitability"
    )
}

/// The pane's opening message: the [`nameplate_art`] splash over the roster
/// hint, which still adapts to provider/roster state (see [`roster`]).
pub(crate) fn startup_banner(reg: &Registry) -> String {
    format!("{}\n\n{}", nameplate_art(), roster(reg))
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
            return emit(out, &msg("agent smith", m));
        }
        let id: Option<u64> = arg.trim_start_matches('#').parse().ok();
        let m = match id {
            Some(id) if tasks.cancel(id) => format!("stopping task #{id}\u{2026}"),
            Some(id) => format!("no task #{id}"),
            None => "usage: /stop [#id]".to_string(),
        };
        return emit(out, &msg("agent smith", m));
    }

    // /tasks — list running tasks.
    if cmd0 == "/tasks" {
        let lines = tasks.describe(Instant::now());
        let body = if lines.is_empty() {
            "no background tasks running".to_string()
        } else {
            lines.join("\n")
        };
        return emit(out, &msg("agent smith", body));
    }

    // /status — session totals plus the LIVE task count (needs the registry,
    // so it's handled here rather than in commands::handle).
    if cmd0 == "/status" {
        return emit(
            out,
            &msg(
                "agent smith",
                super::commands::status_report(session, tasks.len()),
            ),
        );
    }

    // `#note` — remember a standing preference (à la Claude Code's # memory):
    // appended to ./.crew/memory.md and prepended to every task from now on.
    // Answered inline; nothing dials an agent.
    if let Some(note) = trimmed.strip_prefix('#') {
        return emit(out, &msg("agent smith", super::memory::remember(note)));
    }

    if super::commands::is_quick(&trimmed) {
        // Quick constructs never dial an agent (see `is_quick`'s exclusion
        // list), so a no-op tick emitter is correct here, not a shortcut.
        return super::commands::handle(
            session,
            &trimmed,
            &super::tick::noop_tick_emit(),
            &mut |ev| emit(out, &ev),
        );
    }

    if !tasks.admit() {
        return emit(
            out,
            &msg(
                "agent smith",
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
    // No "task started" line: the agent's own streamed output is the
    // acknowledgment (Opencode-style). Only exceptional endings (error/stop)
    // are announced, below.
    let handle = std::thread::spawn(move || {
        let tokens = Arc::clone(&snap.tokens);
        // StatsTicks fire while an agent hop blocks this worker thread — from
        // the provider's own runtime plumbing, not this thread's `counting`
        // closure — so they need their own writer straight to `Out` rather
        // than sharing the `&mut` counting wrapper (ticks are advisory and
        // deliberately skip the per-task token count: the end-of-hop `Stats`
        // stays authoritative).
        let tick_out = Arc::clone(&out_thread);
        let tick_emit: std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync> =
            std::sync::Arc::new(move |ev| {
                let _ = emit(&tick_out, &ev);
            });
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
            super::commands::handle(&mut snap, &trimmed, &tick_emit, &mut counting)
        } else if trimmed.starts_with('@') {
            relay_counting(&trimmed, &snap, &tick_emit, &mut counting)
        } else {
            super::swarm::run_task(&trimmed, &snap, &mut counting)
        };
        // Announce only the exceptional endings — a clean finish says nothing
        // (the streamed reply is the result). Errors and user stops must stay
        // visible.
        let done = match (res, snap.cancelled()) {
            (Err(e), _) => Some(format!("\u{2717} task #{id}: {e}")),
            (Ok(_), true) => Some(format!("\u{2717} task #{id} stopped")),
            (Ok(_), false) => None,
        };
        if let Some(done) = done {
            let _ = emit(&out_thread, &msg("agent smith", done));
        }
    });
    tasks.attach(id, label, cancel, handle, Instant::now());
    Ok(())
}

/// Whether an API provider resolves right now — a key is set (or
/// `CREW_PROVIDER` forces one, or the mock is active). An empty roster means
/// two very different things depending on this: no provider at all, versus a
/// working provider whose per-project specialist store is simply still empty.
pub(crate) fn provider_resolves() -> bool {
    let force = std::env::var("CREW_PROVIDER").ok();
    let has = |k: &str| std::env::var(k).is_ok_and(|v| !v.is_empty());
    super::discover::pick_provider(force.as_deref(), has).is_some()
}

/// A human-readable description of which agents were discovered. When the
/// roster is empty the advice hinges on whether a provider resolves: with no
/// provider the user must set a key; WITH one, the roster is just empty because
/// no swarm has run yet — the planner invents (and records) a team on the first
/// task, so the old "set a key" line would be actively wrong here (a valid key
/// on a fresh project). The provider check reads the live env, same as every
/// other consumer of [`provider_resolves`].
pub(crate) fn roster(reg: &Registry) -> String {
    if reg.is_empty() {
        return if provider_resolves() {
            "No specialists yet — type a task and press Enter; crew assembles a \
             team for it and saves each one, so your @roster grows as you go."
                .into()
        } else {
            "No inbuilt agents available. Set OPENROUTER_API_KEY, \
             DASHSCOPE_API_KEY, or ANTHROPIC_API_KEY and reopen /crew."
                .into()
        };
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
    tick_emit: &std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let reg = session.registry();
    if reg.is_empty() {
        return emit(msg("agent smith", roster(&reg)));
    }
    let task = input.trim();
    if task.is_empty() {
        return Ok(());
    }
    // `@a+b <task>` fans out to that subset in parallel instead of relaying.
    if let Some((names, body)) = multi_targets(task, &reg) {
        emit(msg(
            "agent smith",
            format!("fanning out to {} in parallel\u{2026}", names.join("+")),
        ))?;
        return super::fan::fan_out(&reg, &names, &body, call_timeout(), tick_emit, emit);
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
    // A genuine `@name` dial (not `split_target`'s first-agent fallback for
    // an unaddressed/typo'd task) defers this specialist's LRU eviction, the
    // same way inventing it via a plan does (`specialists::record`) — see
    // `specialists::touch`'s doc. Checked against `task_owned` (post-resume
    // fold), matching exactly what `split_target` below actually consults,
    // so touch and routing never disagree about whether this was a real dial.
    if let Some(name) = dialed_target(&task_owned, &reg) {
        super::specialists::touch(&name);
    }
    let (start, body) = split_target(&task_owned, &reg);
    let tid = format!("t{}", THREAD_SEQ.fetch_add(1, Ordering::Relaxed));
    emit(msg(
        "agent smith",
        format!("starting with {start} — relaying until an agent says @done"),
    ))?;
    let broker = session.broker(reg);
    relay_turn(&broker, &start, &body, &tid, tick_emit, emit).map(|_| ())
}

#[cfg(test)]
#[path = "stdio_tests.rs"]
mod tests;
