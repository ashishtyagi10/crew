//! A relay turn for tests: a scripted CLI agent dialed through `Broker::run`
//! with a tool surface attached, so its `@tool` lines are the harness's text
//! loop to run (`hopturn`), and everything the turn did, read back.
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crew_hive::HiveEvent;

use super::adapter::Adapter;
use super::hop::{Hop, HopKind, RunStats};
use super::Broker;
use crate::{PluginEvent, Registry};

/// An agent that answers from a script, in order, the last reply again once
/// the script runs out (`@done` for an empty one). Keeps every prompt.
pub(crate) struct Script {
    name: &'static str,
    replies: Mutex<VecDeque<Result<String, String>>>,
    prompts: Arc<Mutex<Vec<String>>>,
}

impl Script {
    /// `planner`, answering `replies`.
    pub(crate) fn new(replies: &[&str]) -> Self {
        Self::of(replies.iter().map(|r| Ok(r.to_string())).collect())
    }

    /// `planner`, answering `replies`, an `Err` failing that call.
    pub(crate) fn of(replies: Vec<Result<String, String>>) -> Self {
        Self {
            name: "planner",
            replies: Mutex::new(replies.into()),
            prompts: Arc::default(),
        }
    }

    pub(crate) fn named(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }
}

impl Adapter for Script {
    fn name(&self) -> &str {
        self.name
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: Duration) -> Result<String, String> {
        self.prompts.lock().unwrap().push(body.to_string());
        let mut replies = self.replies.lock().unwrap();
        match replies.len() {
            0 => Ok("@done".into()),
            1 => replies[0].clone(),
            _ => replies.pop_front().unwrap(),
        }
    }
}

/// What one turn did: its hops, the events it sent the pane, every prompt
/// the agent was dialed with, and the thread's stats.
pub(crate) struct Turn {
    pub(crate) hops: Vec<Hop>,
    pub(crate) events: Vec<PluginEvent>,
    pub(crate) prompts: Vec<String>,
    pub(crate) stats: RunStats,
}

impl Turn {
    /// The answer the thread ended on.
    pub(crate) fn answer(&self) -> Option<&str> {
        self.of(HopKind::Done).into_iter().next()
    }

    /// The text of every hop of `kind`.
    pub(crate) fn of(&self, kind: HopKind) -> Vec<&str> {
        self.hops
            .iter()
            .filter(|h| h.kind == kind)
            .map(|h| h.text.as_str())
            .collect()
    }

    /// The tool lane's events, in order.
    pub(crate) fn tools(&self) -> Vec<&HiveEvent> {
        self.events
            .iter()
            .filter_map(|e| match e {
                PluginEvent::Hive { event } => Some(event),
                _ => None,
            })
            .collect()
    }

    /// The tool lane read back as cards: `[tool] label  args` for a call,
    /// `[tool] label ✓|✗` over the text for a result.
    pub(crate) fn cards(&self) -> Vec<String> {
        let mark = |ok: bool| if ok { '\u{2713}' } else { '\u{2717}' };
        self.tools()
            .into_iter()
            .filter_map(|e| match e {
                HiveEvent::ToolCall { label, args, .. } => Some(format!("[tool] {label}  {args}")),
                HiveEvent::ToolResult {
                    label, ok, text, ..
                } => Some(format!("[tool] {label} {}\n{text}", mark(*ok))),
                _ => None,
            })
            .collect()
    }
}

/// One message `body` from the user to `script`, alone on a broker that
/// `setup` gives its tools (and rounds, stop…).
pub(crate) fn turn(script: Script, setup: impl FnOnce(Broker) -> Broker, body: &str) -> Turn {
    let (name, prompts) = (script.name, Arc::clone(&script.prompts));
    let broker = setup(Broker::new(
        Registry::new(vec![Box::new(script)]),
        6,
        Duration::from_secs(5),
    ));
    let events: Arc<Mutex<Vec<PluginEvent>>> = Arc::default();
    let keep = Arc::clone(&events);
    let emit: Arc<dyn Fn(PluginEvent) + Send + Sync> =
        Arc::new(move |e| keep.lock().unwrap().push(e));
    let mut hops = Vec::new();
    let stats = broker.run("user", name, body, "t1", &emit, &mut |h| hops.push(h));
    let events = events.lock().unwrap().clone();
    let prompts = prompts.lock().unwrap().clone();
    Turn {
        hops,
        events,
        prompts,
        stats,
    }
}
