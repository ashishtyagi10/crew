//! The stdio broker loop behind the `/crew` pane. Reads `PluginCommand` JSON
//! lines, discovers the installed agents, relays a task between them, and
//! STREAMS each event as it happens — flushing per line — so the pane shows
//! live progress (activity, each reply) instead of waiting for the whole
//! (slow) relay to finish. Used both by the `crew-broker-plugin` binary and by
//! the `crew` binary re-execing itself with `--broker-plugin`.
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use super::relay::{msg, relay_turn, split_target};
use super::session::Session;
use crate::{Broker, PluginCommand, PluginEvent, Registry};

static THREAD_SEQ: AtomicU64 = AtomicU64::new(1);

fn max_hops() -> u32 {
    env_num("CREW_BROKER_MAX_HOPS").unwrap_or(6)
}
fn call_timeout() -> Duration {
    Duration::from_millis(env_num("CREW_BROKER_TIMEOUT_MS").unwrap_or(180_000))
}
/// Approximate per-thread token budget (0 = unlimited). `CREW_BROKER_TOKEN_BUDGET`.
fn token_budget() -> usize {
    env_num("CREW_BROKER_TOKEN_BUDGET").unwrap_or(0)
}
fn env_num<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok().and_then(|s| s.parse().ok())
}

/// Run the broker over stdin/stdout until EOF, flushing every event so the UI
/// streams progress.
pub fn run_broker_stdio() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    let mut session = Session::new();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(cmd) = serde_json::from_str::<PluginCommand>(&line) else {
            continue;
        };
        dispatch(cmd, &mut out, &mut session)?;
    }
    Ok(())
}

fn emit(out: &mut impl Write, ev: &PluginEvent) -> anyhow::Result<()> {
    writeln!(out, "{}", serde_json::to_string(ev)?)?;
    out.flush()?;
    Ok(())
}

fn dispatch(cmd: PluginCommand, out: &mut impl Write, session: &mut Session) -> anyhow::Result<()> {
    match cmd {
        PluginCommand::Hello { .. } => {
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
            emit(out, &msg("crew", roster(&reg)))?;
        }
        PluginCommand::Send { text, .. } => {
            if super::commands::is_command(&text) {
                super::commands::handle(session, &text, &mut |ev| emit(out, &ev))?;
            } else {
                relay(&text, out, session)?;
            }
        }
        PluginCommand::Subscribe { .. } => {}
    }
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

fn relay(input: &str, out: &mut impl Write, session: &Session) -> anyhow::Result<()> {
    let reg = session.registry();
    if reg.is_empty() {
        return emit(out, &msg("crew", roster(&reg)));
    }
    let task = input.trim();
    if task.is_empty() {
        return Ok(());
    }
    let (start, body) = split_target(task, &reg);
    let tid = format!("t{}", THREAD_SEQ.fetch_add(1, Ordering::Relaxed));
    emit(
        out,
        &msg(
            "crew",
            format!("starting with {start} — relaying until an agent says @done"),
        ),
    )?;
    let broker = Broker::new(reg, max_hops(), call_timeout()).with_budget(token_budget());
    relay_turn(&broker, &start, &body, &tid, &mut |ev| emit(out, &ev))
}

#[cfg(test)]
#[path = "stdio_tests.rs"]
mod tests;
