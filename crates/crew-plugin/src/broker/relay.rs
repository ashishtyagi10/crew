//! One relay turn, as streamed plugin events: live `Activity` while an agent
//! is dialled, each hop as a message, and — when the turn ends — a `Stats`
//! event for the host's token meter plus a per-turn timeline summary line
//! (`turn done — planner 4.2s → coder 8.1s · 2 exchanges · ~950 tok`), timed
//! here at the source so every UI gets the same numbers.
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::{Broker, Hop, HopKind, PluginEvent, Registry};

/// Unix-epoch milliseconds now, as the wire `ts` string.
fn now_ts() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_default()
}

/// Drive one turn of the broker over `emit`, timing each agent call.
pub(crate) fn relay_turn(
    broker: &Broker,
    start: &str,
    body: &str,
    tid: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut timing: Option<(String, Instant)> = None;
    let mut segments: Vec<(String, Duration)> = Vec::new();
    let mut werr: anyhow::Result<()> = Ok(());
    let stats = broker.run("user", start, body, tid, &mut |hop| {
        // A Dialing hop opens an agent's segment; any other hop closes it —
        // the closed segment is that reply's latency.
        let mut latency = None;
        match (&hop.kind, timing.take()) {
            (HopKind::Dialing, prev) => {
                if let Some((agent, t0)) = prev {
                    segments.push((agent, t0.elapsed()));
                }
                timing = Some((hop.to.clone(), Instant::now()));
            }
            (_, Some((agent, t0))) => {
                let d = t0.elapsed();
                latency = Some(d);
                segments.push((agent, d));
            }
            _ => {}
        }
        if werr.is_ok() {
            werr = emit(hop_to_msg(&hop, latency));
        }
    });
    werr?;
    if let Some((agent, t0)) = timing.take() {
        segments.push((agent, t0.elapsed()));
    }
    emit(PluginEvent::Stats {
        exchanges: stats.exchanges,
        tokens: stats.approx_tokens as u64,
    })?;
    emit(msg(
        "crew",
        turn_summary(&segments, stats.exchanges, stats.approx_tokens),
    ))?;
    emit(PluginEvent::Activity {
        agent: String::new(),
        state: "idle".into(),
    })
}

/// The per-turn log line: who worked for how long, and what it cost.
pub(crate) fn turn_summary(
    segments: &[(String, Duration)],
    exchanges: u32,
    approx_tokens: usize,
) -> String {
    let timeline: Vec<String> = segments
        .iter()
        .map(|(agent, d)| format!("{agent} {:.1}s", d.as_secs_f32()))
        .collect();
    let head = if timeline.is_empty() {
        "turn done".to_string()
    } else {
        format!("turn done \u{2014} {}", timeline.join(" \u{2192} "))
    };
    format!("{head} \u{00b7} {exchanges} exchange(s) \u{00b7} ~{approx_tokens} tok (approx)")
}

/// Render a hop as a plugin event. `Dialing` becomes a live `Activity` status
/// (the agent is thinking) rather than transcript spam; every other hop is a
/// message labelled `from → to`, carrying the reply's latency as its metadata.
pub(crate) fn hop_to_msg(hop: &Hop, latency: Option<Duration>) -> PluginEvent {
    match hop.kind {
        HopKind::Dialing => PluginEvent::Activity {
            agent: hop.to.clone(),
            state: "thinking".into(),
        },
        _ => PluginEvent::Message {
            channel: "crew".into(),
            sender: format!("{} \u{2192} {}", hop.from, hop.to),
            text: hop_text(hop),
            ts: now_ts(),
            meta: latency
                .map(|d| format!("{:.1}s", d.as_secs_f32()))
                .unwrap_or_default(),
        },
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

pub(crate) fn msg(sender: &str, text: impl Into<String>) -> PluginEvent {
    PluginEvent::Message {
        channel: "crew".into(),
        sender: sender.into(),
        text: text.into(),
        ts: now_ts(),
        meta: String::new(),
    }
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

#[cfg(test)]
#[path = "relay_tests.rs"]
mod tests;
