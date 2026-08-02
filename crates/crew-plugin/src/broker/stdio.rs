//! The stdio broker loop behind the `/crew` pane. Reads `PluginCommand` JSON
//! lines and STREAMS every event as it happens (flushing per line). Long work
//! — a relay turn, /fan, /loop, /goal — spawns a **background task** (its own
//! worker thread, id, and cancel flag) so several run at once (up to
//! `CREW_MAX_TASKS`) while the main loop keeps draining stdin. Quick constructs
//! (/help, /model, …) answer inline; the pane's footer lists the running
//! tasks (see `PluginEvent::Task`) and `/stop [#n]` cancels all of them or
//! just task #n between hops/rounds. Each
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
    emit(out, &msg("agent smith", startup_banner(&reg)))?;
    // …and, if yesterday's conversation is still here, that it is. Held back
    // until after the banner so the pane's own identity reads first.
    match super::sessionlog::resume_offer() {
        Some(note) => emit(out, &msg("agent smith", note)),
        None => Ok(()),
    }
}

/// Smith's dialog pool: the tagline under the nameplate, one line per pane.
/// Every pane spawns its own broker process, so seeding off the wall clock at
/// startup gives each new pane a different opening line.
const SMITH_LINES: &[&str] = &[
    "the sound of inevitability",
    "never send a human to do a machine's job",
    "it is purpose that created us",
    "we're not here because we're free",
    "everything that has a beginning has an end",
    "you hear that, Mr. Anderson?",
    "the best thing about being me\u{2026} there are so many mes",
];

/// One line from [`SMITH_LINES`], picked at broker start.
fn smith_line() -> &'static str {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() ^ u64::from(d.subsec_nanos()))
        .unwrap_or(0);
    SMITH_LINES[(seed % SMITH_LINES.len() as u64) as usize]
}

/// The Agent Smith opening splash — a boxed nameplate over a binary "code
/// rain" tagline drawn from his dialog pool ([`smith_line`]), in the spirit
/// of the opencode/claude/codex startup banners. Rendered in the theme ink
/// (Matrix green on the CRT themes). Kept ASCII-narrow so it survives slim
/// panes; the chat renderer preserves its line breaks (soft breaks are hard
/// breaks in chat), so the box stays intact.
fn nameplate_art() -> String {
    let plate = "A G E N T   S M I T H";
    let pad = 3;
    let bar = "\u{2550}".repeat(plate.chars().count() + pad * 2);
    let sp = " ".repeat(pad);
    format!(
        "\u{2554}{bar}\u{2557}\n\
         \u{2551}{sp}{plate}{sp}\u{2551}\n\
         \u{255a}{bar}\u{255d}\n\
         01001101 \u{22ee} {}",
        smith_line()
    )
}

/// The pane's opening message: the [`nameplate_art`] splash alone — the pane
/// opens on just the centered nameplate, no roster chatter. The one exception
/// is a dead-on-arrival session (no provider key at all), where the [`roster`]
/// warning still rides below the art because without it the user has no way
/// to know why nothing will ever answer.
pub(crate) fn startup_banner(reg: &Registry) -> String {
    banner_for(reg, provider_resolves())
}

/// [`startup_banner`] with the provider verdict passed in.
///
/// Split out because the verdict comes from process-wide state — env vars and
/// an on-disk credential store — that other tests mutate as they run. A test
/// calling `provider_resolves()` to decide what to assert therefore exercises
/// whichever branch the scheduler hands it, which is how a warning string that
/// changed in v0.6.39 went on passing for five releases: the assertion naming
/// the old text simply was not reached. With the verdict as a parameter, both
/// branches are checked on every run and neither can rot unobserved.
fn banner_for(reg: &Registry, provider: bool) -> String {
    if reg.is_empty() && !provider {
        return format!("{}\n\n{}", nameplate_art(), roster_for(reg, provider));
    }
    nameplate_art()
}

/// Route one Send. `/stop [#N]` and quick constructs
/// answer inline; every other task spawns a NEW background worker (up to the
/// cap), so several run at once.
fn send(
    text: String,
    out: &Out,
    session: &mut Session,
    tasks: &mut super::tasks::Tasks,
) -> anyhow::Result<()> {
    use std::sync::atomic::AtomicBool;
    tasks.reap();
    let trimmed = text.trim().to_string();
    // Resolve built-in single-letter aliases (`/d` → `/diff`) before ANY
    // routing below, so they reach the same interceptors their long form does.
    let trimmed = super::commands::expand_alias(&trimmed);

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
    let label_for_ckpt = label.clone();
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
        // Undo, without anyone having to remember to ask for it. Every task
        // that reaches a worker can reach the user's files (sys tools, an
        // agent CLI editing in place), so the working tree is pinned first.
        // Silent by design: it writes nothing when nothing changed, and a
        // safety net that announced itself on every task would be noise. Not
        // being in a git repository is a normal way to run crew, so the error
        // is deliberately ignored rather than reported.
        auto_checkpoint(&snap, &label_for_ckpt, &out_thread);
        let res = if is_cmd {
            super::commands::handle(&mut snap, &trimmed, &tick_emit, &mut counting)
        } else if trimmed.starts_with('@') {
            relay_counting(&trimmed, &snap, &tick_emit, &mut counting)
        } else if let Some(starter) = super::auth::starter(&snap) {
            relay_counting(
                &format!("@{starter} {trimmed}"),
                &snap,
                &tick_emit,
                &mut counting,
            )
        } else {
            // The intent router: the model picks the execution shape (reply/
            // fan/loop/plan/swarm); anything that stops it falls back to the
            // swarm, which was this branch's whole body before the router.
            super::intent::route(&trimmed, &mut snap, &tick_emit, &mut counting)
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
        // …and then what it DID to the files, which nothing on screen said.
        report_changes(&snap, id, &out_thread);
        // The end of the task, emitted by the only thing that knows when that
        // is. `Tasks::reap` runs lazily on the NEXT command, so a task
        // finishing is otherwise not an observable moment — which is why the
        // pane could never show what was running and had to be asked.
        let _ = emit(
            &out_thread,
            &PluginEvent::Task {
                id,
                label: String::new(),
                running: false,
            },
        );
    });
    emit(
        out,
        &PluginEvent::Task {
            id,
            label: label.clone(),
            running: true,
        },
    )?;
    tasks.attach(id, cancel, handle);
    Ok(())
}

/// Whether an API provider resolves right now — a key is set (or
/// `CREW_PROVIDER` forces one, or the mock is active). An empty roster means
/// two very different things depending on this: no provider at all, versus a
/// working provider whose per-project specialist store is simply still empty.
pub(crate) fn provider_resolves() -> bool {
    // Through `resolved_provider`, so a key supplied from inside crew (stored,
    // never exported into this process) counts here exactly as it counts when
    // the roster is actually built — otherwise this would tell the user to set
    // a key they had already typed in.
    super::discover::resolved_provider().is_some()
}

/// A human-readable description of which agents were discovered. When the
/// roster is empty the advice hinges on whether a provider resolves: with no
/// provider the user must set a key; WITH one, the roster is just empty because
/// no swarm has run yet — the planner invents (and records) a team on the first
/// task, so the old "set a key" line would be actively wrong here (a valid key
/// on a fresh project). The provider check reads the live env, same as every
/// other consumer of [`provider_resolves`].
pub(crate) fn roster(reg: &Registry) -> String {
    roster_for(reg, provider_resolves())
}

/// [`roster`] with the provider verdict passed in, for the same reason
/// [`banner_for`] takes one: the verdict is process-wide state, and a caller
/// that has already decided must not have that decision silently re-made
/// underneath it by an env var another thread changed mid-call.
pub(crate) fn roster_for(reg: &Registry, provider: bool) -> String {
    if reg.is_empty() {
        return if provider {
            "No specialists yet — type a task and press Enter; crew assembles a \
             team for it and saves each one, so your @roster grows as you go."
                .into()
        } else {
            // One copy of this advice, shared with `/doctor` and the Far
            // pane's ask (`discover::no_provider_advice`). Four wordings of it
            // existed and two went stale for two releases when a signed-in CLI
            // became a valid answer.
            format!(
                "No agents available — {}.",
                super::discover::no_provider_advice()
            )
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

/// Pin the working tree before a task runs, so anything it changes can be put
/// back. Best-effort and silent: `auto_snapshot` writes nothing when the tree
/// is unchanged since the last one, and a machine outside a git repository —
/// a normal way to run crew — simply has no checkpoints.
///
/// This replaced `/checkpoint`. Asking a user to predict which task is the one
/// worth protecting is asking them to be right in advance; the safety net is
/// worth more when nobody has to think about it.
fn auto_checkpoint(session: &Session, label: &str, out: &Out) {
    let Ok(dir) = std::env::current_dir() else {
        return;
    };
    let took = {
        let mut last = session.last_tree.lock().unwrap_or_else(|e| e.into_inner());
        super::checkpoint::auto_snapshot(&dir, &format!("before: {label}"), &mut last)
    };
    // Say it ONCE, the first time one is actually written. Before that there
    // is nothing to restore and the note would be a promise; after it, a note
    // on every task would be the noise this feature stays silent to avoid.
    if matches!(took, Ok(Some(_))) && !announced(session) {
        let _ = emit(
            out,
            &msg(
                "agent smith",
                "snapshot taken before this task \u{2014} /restore lists them, \
                 /restore <n> puts one back",
            ),
        );
    }
}

/// Say which files the task changed, once it has finished.
///
/// A clean run prints its reply and nothing else, so an agent that edited four
/// files and answered "done" left no record anywhere of WHICH four — the user
/// had to know to run `/diff` and remember to do it. The pane reports what it
/// is doing everywhere else (the footer's tasks, its cwd, its roster); this is
/// the same principle applied to the thing that actually matters.
///
/// The comparison is free of new bookkeeping: `auto_checkpoint` has just left
/// the pre-task tree in `session.last_tree`, so this is one diff against it.
/// `last_tree` is deliberately NOT updated — it must stay at the pre-task tree
/// so the NEXT task's checkpoint captures what this one wrote.
///
/// Silent on every failure, for the reason the checkpoint is: running outside
/// a git repository is normal, and a safety net that announces its own absence
/// is noise.
fn report_changes(session: &Session, id: u64, out: &Out) {
    let Ok(dir) = std::env::current_dir() else {
        return;
    };
    let base = {
        let last = session.last_tree.lock().unwrap_or_else(|e| e.into_inner());
        last.clone()
    };
    let Some(base) = base else { return };
    let Ok(changes) = super::changed::since(&dir, &base) else {
        return;
    };
    // Checked BEFORE the flag is taken: a run of questions that touch no
    // files must not spend the one-time hint on notes nobody ever saw.
    if changes.is_empty() {
        return;
    }
    let hint = !session
        .announced_changes
        .swap(true, std::sync::atomic::Ordering::Relaxed);
    if let Some(line) = super::changed::summary(&changes, hint) {
        let _ = emit(out, &msg("agent smith", format!("task #{id}: {line}")));
    }
}

/// Whether this session has already mentioned checkpoints; marks it as told.
fn announced(session: &Session) -> bool {
    session
        .announced_ckpt
        .swap(true, std::sync::atomic::Ordering::Relaxed)
}

/// Who should answer a plain (unaddressed) task when there is no API provider.
///
/// The swarm cannot run without one — planning is an LLM call, so keyless
/// `run_task` falls back to `StubPlanner` and answers "stub:0". That was
/// tolerable while a keyless machine had no agents at all; it is not tolerable
/// now that a logged-in `claude`/`codex`/`opencode` install puts real agents
/// on the roster, because the user would see three capable agents and get
/// stub output. When there is someone who can actually answer, relay to them
/// instead — which is exactly what typing `@claude <task>` already does, minus
/// the requirement that the user know to type it.
///
/// `provider_resolves` is checked FIRST and short-circuits: it reads the
/// credential store, while `registry()` re-runs discovery (manifest reads plus
/// PATH probes). Anyone with a key pays nothing for this.
pub(crate) fn keyless_starter(session: &Session) -> Option<String> {
    if provider_resolves() {
        return None;
    }
    first_starter(session.registry().names())
}

/// The pure half of [`keyless_starter`]: who leads, given the roster. First
/// registered wins — `roster_with` orders manifest agents ahead of built-ins,
/// so a user's own declaration leads if they wrote one, and otherwise it is
/// `claude` ("planning, analysis, prose"), the right default to open on.
/// `None` when nobody is there, which leaves the stub swarm exactly as it was.
fn first_starter(names: Vec<String>) -> Option<String> {
    names.into_iter().next()
}

/// `pub(crate)` for the intent router: a `reply`-shaped plain message takes
/// exactly the path an `@agent` dial takes, minus the dial.
pub(crate) fn relay_counting(
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
