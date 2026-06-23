//! The `/crew` broker plugin: a JSON-line subprocess that discovers the coding
//! agents installed on this machine (claude, codex, opencode) and relays a task
//! between them, emitting one `Message` event per hop. Every slow agent call
//! happens here — off Crew's render thread — so the pane only ever polls events.
use crew_plugin::{Broker, Hop, HopKind, PluginCommand, PluginEvent, Registry};
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static THREAD_SEQ: AtomicU64 = AtomicU64::new(1);

/// Maximum relay depth before the broker drops a thread (loop guard).
/// Overridable via `CREW_BROKER_MAX_HOPS` (used by the e2e tests).
fn max_hops() -> u32 {
    env_num("CREW_BROKER_MAX_HOPS").unwrap_or(6)
}

/// Per-call timeout: a hung agent is killed and logged after this long.
/// Overridable via `CREW_BROKER_TIMEOUT_MS`.
fn call_timeout() -> Duration {
    Duration::from_millis(env_num("CREW_BROKER_TIMEOUT_MS").unwrap_or(180_000))
}

fn env_num<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok().and_then(|s| s.parse().ok())
}

fn main() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(cmd) = serde_json::from_str::<PluginCommand>(&line) else {
            continue;
        };
        for ev in handle(cmd) {
            writeln!(out, "{}", serde_json::to_string(&ev)?)?;
        }
        out.flush()?;
    }
    Ok(())
}

/// Handle one command into the events to emit. `Send` blocks while the relay
/// runs; that is fine in this subprocess (it never touches the UI).
fn handle(cmd: PluginCommand) -> Vec<PluginEvent> {
    match cmd {
        PluginCommand::Hello { .. } => hello(),
        PluginCommand::Send { text, .. } => relay(&text),
        PluginCommand::Subscribe { .. } => vec![],
    }
}

fn hello() -> Vec<PluginEvent> {
    let reg = Registry::discover();
    vec![
        PluginEvent::Ready {
            v: 1,
            provider: "crew".into(),
            channels: vec!["crew".into()],
        },
        msg("crew", roster(&reg)),
    ]
}

/// A human-readable description of which agents were discovered.
fn roster(reg: &Registry) -> String {
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

fn relay(input: &str) -> Vec<PluginEvent> {
    let reg = Registry::discover();
    if reg.is_empty() {
        return vec![msg("crew", roster(&reg))];
    }
    let task = input.trim();
    if task.is_empty() {
        return vec![];
    }
    let (start, body) = split_target(task, &reg);
    let tid = format!("t{}", THREAD_SEQ.fetch_add(1, Ordering::Relaxed));
    let mut events = vec![msg("crew", format!("→ starting with {start}"))];
    let broker = Broker::new(reg, max_hops(), call_timeout());
    broker.run("user", &start, &body, &tid, &mut |hop| {
        events.push(msg(&format!("{} → {}", hop.from, hop.to), hop_text(&hop)));
    });
    events
}

/// Split an optional leading `@agent` selector off the task. Falls back to the
/// first discovered agent when no valid selector is present.
fn split_target(task: &str, reg: &Registry) -> (String, String) {
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

/// Prefix a hop's text with a marker for non-reply outcomes.
fn hop_text(hop: &Hop) -> String {
    match hop.kind {
        HopKind::Reply => hop.text.clone(),
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
mod tests {
    use super::*;
    use crew_plugin::{CliAdapter, Normalize};

    fn reg(names: &[&str]) -> Registry {
        Registry::new(
            names
                .iter()
                .map(|n| {
                    Box::new(CliAdapter {
                        name: (*n).into(),
                        program: "true".into(),
                        args: vec![],
                        normalize: Normalize::Raw,
                    }) as Box<dyn crew_plugin::Adapter>
                })
                .collect(),
        )
    }

    #[test]
    fn split_target_defaults_to_first_agent() {
        let (s, b) = split_target("do the thing", &reg(&["claude", "codex"]));
        assert_eq!(s, "claude");
        assert_eq!(b, "do the thing");
    }

    #[test]
    fn split_target_honours_at_selector() {
        let (s, b) = split_target("@codex review this", &reg(&["claude", "codex"]));
        assert_eq!(s, "codex");
        assert_eq!(b, "review this");
    }

    #[test]
    fn split_target_ignores_unknown_selector() {
        let (s, b) = split_target("@ghost hi", &reg(&["claude"]));
        assert_eq!(s, "claude");
        assert_eq!(b, "@ghost hi");
    }

    #[test]
    fn hop_text_marks_outcomes() {
        let h = |kind, text: &str| Hop {
            from: "a".into(),
            to: "b".into(),
            hop: 0,
            kind,
            text: text.into(),
        };
        assert_eq!(hop_text(&h(HopKind::Reply, "hi")), "hi");
        assert_eq!(hop_text(&h(HopKind::Done, "")), "[done]");
        assert_eq!(hop_text(&h(HopKind::Error, "x")), "[error] x");
        assert_eq!(hop_text(&h(HopKind::Terminated, "y")), "[stopped] y");
    }

    #[test]
    fn roster_lists_agents_or_explains_absence() {
        assert!(roster(&reg(&["claude", "codex"])).contains("claude, codex"));
        assert!(roster(&reg(&[])).contains("No coding agents"));
    }
}
