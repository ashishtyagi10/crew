//! Parallel fan-out: send one task to several agents **concurrently** (one OS
//! thread per call via `std::thread::scope`) and stream each reply back the
//! moment it lands — fastest agent first — followed by a combined `Stats`
//! event and a per-agent timing summary. This is the `/fan` construct and the
//! machinery behind multi-target `@a+b` sends.
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::{PluginEvent, Registry, Routing};

use super::relay::msg;

/// Send `task` to each of `names` in parallel; every reply/error is emitted as
/// it arrives, then a `Stats` event and a summary line close the turn.
pub(crate) fn fan_out(
    reg: &Registry,
    names: &[String],
    task: &str,
    timeout: Duration,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
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
            s.spawn(move || {
                let t0 = Instant::now();
                let res = agent.call(&prompt, timeout);
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
            let ev = match res {
                Ok(reply) => {
                    tokens += (prompt.len() + reply.len()) / 4;
                    timings.push((name.clone(), dt));
                    reply_msg(&name, &reply, dt)
                }
                Err(e) => msg(&format!("{name} \u{2192} user"), format!("[error] {e}")),
            };
            if werr.is_ok() {
                werr = emit(done).and_then(|()| emit(ev));
            }
        }
    });
    werr?;
    timings.sort_by_key(|(_, d)| *d);
    let order: Vec<String> = timings
        .iter()
        .map(|(n, d)| format!("{n} {:.1}s", d.as_secs_f32()))
        .collect();
    emit(PluginEvent::Stats {
        exchanges: names.len() as u32,
        tokens: tokens as u64,
    })?;
    emit(msg(
        "crew",
        format!(
            "fan done \u{2014} {} of {} replied \u{2225} {} \u{00b7} ~{tokens} tok (approx)",
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
