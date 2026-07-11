//! Parallel fan-out: send one task to several agents **concurrently** (one OS
//! thread per call via `std::thread::scope`) and stream each reply back the
//! moment it lands — fastest agent first — followed by a combined `Stats`
//! event and a per-agent timing summary. This is the `/fan` construct and the
//! machinery behind multi-target `@a+b` sends.
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::{PluginEvent, Registry, Routing};

use super::relay::msg;
use super::tick::hop_ticker;

/// Send `task` to each of `names` in parallel; every reply/error is emitted as
/// it arrives, then a `Stats` event and a summary line close the turn.
pub(crate) fn fan_out(
    reg: &Registry,
    names: &[String],
    task: &str,
    timeout: Duration,
    tick_emit: &Arc<dyn Fn(PluginEvent) + Send + Sync>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    // Standing memory rides fan tasks too (see relay_turn).
    let task = &super::memory::with_memory(task);
    // Every agent starts thinking at once, each on the user's behalf.
    for name in names {
        emit(PluginEvent::Activity {
            agent: name.clone(),
            state: "thinking".into(),
            from: "user".into(),
        })?;
    }
    let prompt = format!(
        "Answer the task directly and concisely. Do NOT include an `@next` or \
         `@done` control line.\n\nTask: {task}"
    );
    let mut tokens = 0usize;
    let mut real_tokens = 0usize;
    let mut timings: Vec<(String, Duration)> = Vec::new();
    let mut werr: anyhow::Result<()> = Ok(());
    std::thread::scope(|s| {
        let (tx, rx) = mpsc::channel();
        for name in names {
            let Some(agent) = reg.get(name) else {
                let _ = tx.send((
                    name.clone(),
                    Err(format!("unknown agent \"{name}\"")),
                    Duration::ZERO,
                ));
                continue;
            };
            let tx = tx.clone();
            let prompt = prompt.clone();
            let on_tokens = hop_ticker(tick_emit.clone(), name.clone());
            s.spawn(move || {
                let t0 = Instant::now();
                let res = agent.call_with_usage_ticked(&prompt, timeout, on_tokens);
                let _ = tx.send((name.clone(), res, t0.elapsed()));
            });
        }
        drop(tx); // scope's own sender gone → rx ends when all workers finish
        for (name, res, dt) in rx {
            // Each agent goes idle as its reply lands (the pane tracks the set).
            let done = PluginEvent::Activity {
                agent: name.clone(),
                state: "idle".into(),
                from: String::new(),
            };
            let (ev, stat) = match res {
                Ok((reply, u)) => {
                    tokens += (prompt.len() + reply.len()) / 4;
                    real_tokens += (u.input_tokens + u.output_tokens) as usize;
                    timings.push((name.clone(), dt));
                    // The agent's live reply stat, real usage when reported.
                    let stat = PluginEvent::Stats {
                        exchanges: 0,
                        tokens: (u.input_tokens + u.output_tokens) as u64,
                        agent: name.clone(),
                        ms: dt.as_millis() as u64,
                        ctx: u.input_tokens as u64,
                    };
                    (reply_msg(&name, &reply, dt), Some(stat))
                }
                Err(e) => {
                    // Even on failure, close this agent's dial with a
                    // zero-usage Stats so the tok display reconciles and the
                    // reply lifecycle doesn't stay open — mirroring how
                    // relay.rs closes every hop, including HopKind::Error.
                    let stat = PluginEvent::Stats {
                        exchanges: 0,
                        tokens: 0,
                        agent: name.clone(),
                        ms: dt.as_millis() as u64,
                        ctx: 0,
                    };
                    (
                        msg(&format!("{name} \u{2192} user"), format!("[error] {e}")),
                        Some(stat),
                    )
                }
            };
            if werr.is_ok() {
                werr = emit(done)
                    .and_then(|()| match stat {
                        Some(s) => emit(s),
                        None => Ok(()),
                    })
                    .and_then(|()| emit(ev));
            }
        }
    });
    werr?;
    timings.sort_by_key(|(_, d)| *d);
    let order: Vec<String> = timings
        .iter()
        .map(|(n, d)| format!("{n} {:.1}s", d.as_secs_f32()))
        .collect();
    let (total, approx) = if real_tokens > 0 {
        (real_tokens, false)
    } else {
        (tokens, true)
    };
    let cost = if approx {
        format!("~{total} tok (approx)")
    } else {
        format!("{total} tok")
    };
    emit(PluginEvent::Stats {
        exchanges: names.len() as u32,
        tokens: total as u64,
        agent: String::new(),
        ms: 0,
        ctx: 0,
    })?;
    emit(msg(
        "crew",
        format!(
            "fan done \u{2014} {} of {} replied \u{2225} {} \u{00b7} {cost}",
            timings.len(),
            names.len(),
            order.join(" \u{00b7} "),
        ),
    ))?;
    emit(PluginEvent::Activity {
        agent: String::new(),
        state: "idle".into(),
        from: String::new(),
    })
}

/// An agent's fan reply as a chat message, control lines stripped, latency in
/// the metadata.
fn reply_msg(name: &str, reply: &str, dt: Duration) -> PluginEvent {
    let clean = match crate::parse_routing(reply) {
        Routing::Done(body) | Routing::Relay { body, .. } if !body.is_empty() => body,
        _ => reply.trim().to_string(),
    };
    match msg(&format!("{name} \u{2192} user"), clean) {
        PluginEvent::Message {
            channel,
            sender,
            text,
            ts,
            ..
        } => PluginEvent::Message {
            channel,
            sender,
            text,
            ts,
            meta: format!("{:.1}s", dt.as_secs_f32()),
        },
        ev => ev,
    }
}

#[cfg(test)]
#[path = "fan_tests.rs"]
mod tests;
