//! The stdio broker loop behind the `/crew` pane. Reads `PluginCommand` JSON
//! lines and STREAMS every event as it happens (flushing per line). Long work
//! — a relay turn, /fan, /loop, /goal — runs on a **worker thread** so the
//! main loop keeps draining stdin: quick constructs (/help, /model, …) answer
//! immediately and `/stop` can cancel the running task between hops/rounds.
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
    let mut o = out.lock().unwrap_or_else(|e| e.into_inner());
    writeln!(o, "{}", serde_json::to_string(ev)?)?;
    o.flush()?;
    Ok(())
}

/// Run the broker over stdin/stdout until EOF.
pub fn run_broker_stdio() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let out: Out = Arc::new(Mutex::new(std::io::stdout()));
    let mut session = Session::new();
    let mut worker: Option<std::thread::JoinHandle<()>> = None;
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(cmd) = serde_json::from_str::<PluginCommand>(&line) else {
            continue;
        };
        match cmd {
            PluginCommand::Hello { .. } => hello(&out, &session)?,
            PluginCommand::Send { text, .. } => send(text, &out, &mut session, &mut worker)?,
            PluginCommand::Subscribe { .. } => {}
        }
    }
    // stdin closed (pane gone / EOF): let a running task finish streaming
    // rather than truncating its output mid-line.
    if let Some(h) = worker {
        let _ = h.join();
    }
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

/// Route one Send: `/stop` and quick constructs answer inline; tasks and long
/// constructs run on the worker thread (one at a time).
fn send(
    text: String,
    out: &Out,
    session: &mut Session,
    worker: &mut Option<std::thread::JoinHandle<()>>,
) -> anyhow::Result<()> {
    let trimmed = text.trim().to_string();
    let running = session.running();
    if trimmed == "/stop" {
        return match running {
            Some(label) => {
                session.cancel.store(true, Ordering::Relaxed);
                emit(
                    out,
                    &msg("crew", format!("stopping \u{2018}{label}\u{2019}\u{2026}")),
                )
            }
            None => emit(out, &msg("crew", "nothing is running")),
        };
    }
    if super::commands::is_quick(&trimmed) {
        return super::commands::handle(session, &trimmed, &mut |ev| emit(out, &ev));
    }
    if let Some(label) = running {
        return emit(
            out,
            &msg(
                "crew",
                format!("busy with \u{2018}{label}\u{2019} \u{2014} /stop cancels it"),
            ),
        );
    }
    session.cancel.store(false, Ordering::Relaxed);
    session.turns.fetch_add(1, Ordering::Relaxed);
    let label: String = trimmed.chars().take(40).collect();
    *session.busy.lock().unwrap_or_else(|e| e.into_inner()) = Some(label);
    let mut snap = session.snapshot();
    let out = Arc::clone(out);
    // A finished worker's handle may be overwritten here (it's already done;
    // dropping the handle just detaches the dead thread).
    *worker = Some(std::thread::spawn(move || {
        let tokens = Arc::clone(&snap.tokens);
        let busy = Arc::clone(&snap.busy);
        // Count every Stats event into the session totals for /status.
        let mut counting = |ev: PluginEvent| {
            if let PluginEvent::Stats { tokens: t, .. } = &ev {
                tokens.fetch_add(*t, Ordering::Relaxed);
            }
            emit(&out, &ev)
        };
        let res = if super::commands::is_command(&trimmed) {
            super::commands::handle(&mut snap, &trimmed, &mut counting)
        } else {
            relay_counting(&trimmed, &snap, &mut counting)
        };
        if let Err(e) = res {
            eprintln!("crew-broker: worker error: {e}");
        }
        *busy.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }));
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
    let (start, body) = split_target(task, &reg);
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
