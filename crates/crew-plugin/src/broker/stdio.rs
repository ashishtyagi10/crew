//! The stdio broker loop behind the `/crew` pane. Reads `PluginCommand` JSON
//! lines, discovers the installed agents, relays a task between them, and
//! STREAMS each event as it happens — flushing per line — so the pane shows
//! live progress ("calling codex…", each reply) instead of waiting for the
//! whole (slow) relay to finish. Used both by the `crew-broker-plugin` binary
//! and by the `crew` binary re-execing itself with `--broker-plugin`.
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::{Broker, Hop, HopKind, PluginCommand, PluginEvent, Registry};

static THREAD_SEQ: AtomicU64 = AtomicU64::new(1);

fn max_hops() -> u32 {
    env_num("CREW_BROKER_MAX_HOPS").unwrap_or(6)
}
fn call_timeout() -> Duration {
    Duration::from_millis(env_num("CREW_BROKER_TIMEOUT_MS").unwrap_or(180_000))
}
fn env_num<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok().and_then(|s| s.parse().ok())
}

/// Run the broker over stdin/stdout until EOF, flushing every event so the UI
/// streams progress.
pub fn run_broker_stdio() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(cmd) = serde_json::from_str::<PluginCommand>(&line) else {
            continue;
        };
        dispatch(cmd, &mut out)?;
    }
    Ok(())
}

fn emit(out: &mut impl Write, ev: &PluginEvent) -> anyhow::Result<()> {
    writeln!(out, "{}", serde_json::to_string(ev)?)?;
    out.flush()?;
    Ok(())
}

fn dispatch(cmd: PluginCommand, out: &mut impl Write) -> anyhow::Result<()> {
    match cmd {
        PluginCommand::Hello { .. } => {
            let reg = Registry::discover();
            emit(
                out,
                &PluginEvent::Ready {
                    v: 1,
                    provider: "crew".into(),
                    channels: vec!["crew".into()],
                },
            )?;
            emit(out, &msg("crew", roster(&reg)))?;
        }
        PluginCommand::Send { text, .. } => relay(&text, out)?,
        PluginCommand::Subscribe { .. } => {}
    }
    Ok(())
}

/// A human-readable description of which agents were discovered.
pub(crate) fn roster(reg: &Registry) -> String {
    if reg.is_empty() {
        return "No coding agents found on PATH. Install claude, codex, or \
                opencode and reopen /crew."
            .into();
    }
    format!(
        "Detected {} agent(s): {}. Type a task and press Enter; prefix @<agent> \
         to choose who starts. Agents hand off with \"TO <agent>:\" and finish \
         with \"DONE\".",
        reg.len(),
        reg.names().join(", "),
    )
}

fn relay(input: &str, out: &mut impl Write) -> anyhow::Result<()> {
    let reg = Registry::discover();
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
            format!("starting with {start} — relaying until an agent says DONE"),
        ),
    )?;
    let broker = Broker::new(reg, max_hops(), call_timeout());
    let mut werr: anyhow::Result<()> = Ok(());
    broker.run("user", &start, &body, &tid, &mut |hop| {
        if werr.is_ok() {
            werr = emit(out, &hop_to_msg(&hop));
        }
    });
    werr
}

/// Split an optional leading `@agent` selector off the task. Falls back to the
/// first discovered agent when no valid selector is present.
pub(crate) fn split_target(task: &str, reg: &Registry) -> (String, String) {
    let default = reg.names().into_iter().next().unwrap_or_default();
    if let Some(rest) = task.strip_prefix('@') {
        if let Some((name, body)) = rest.split_once(char::is_whitespace) {
            if reg.get(name).is_some() {
                return (name.to_string(), body.trim().to_string());
            }
        }
    }
    (default, task.to_string())
}

/// Render a hop as a chat message. `Dialing` is a `crew` progress note naming
/// the agent being called; every other hop is labelled `from → to`.
pub(crate) fn hop_to_msg(hop: &Hop) -> PluginEvent {
    match hop.kind {
        HopKind::Dialing => msg(
            "crew",
            format!("calling {}… (waiting for its reply)", hop.to),
        ),
        _ => msg(&format!("{} → {}", hop.from, hop.to), hop_text(hop)),
    }
}

fn hop_text(hop: &Hop) -> String {
    match hop.kind {
        HopKind::Dialing | HopKind::Reply => hop.text.clone(),
        HopKind::Done if hop.text.is_empty() => "[done]".into(),
        HopKind::Done => format!("[done] {}", hop.text),
        HopKind::Terminated => format!("[stopped] {}", hop.text),
        HopKind::Error => format!("[error] {}", hop.text),
    }
}

fn msg(sender: &str, text: impl Into<String>) -> PluginEvent {
    PluginEvent::Message {
        channel: "crew".into(),
        sender: sender.into(),
        text: text.into(),
        ts: String::new(),
    }
}

#[cfg(test)]
#[path = "stdio_tests.rs"]
mod tests;
