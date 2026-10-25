//! A scripted agent and a session surface that keeps a checklist, for the
//! relay's checklist tests.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use crate::broker::adapter::Adapter;
use crate::broker::hop::HopKind;
use crate::Registry;
use crew_hive::tools::todo::Checklist;

/// Replies in order, then `@done`; keeps every prompt it was dialed with.
struct Scripted(Mutex<Vec<String>>, Arc<Mutex<Vec<String>>>);

impl Adapter for Scripted {
    fn name(&self) -> &str {
        "coder"
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, _t: Duration) -> Result<String, String> {
        self.1.lock().unwrap().push(body.to_string());
        Ok(self
            .0
            .lock()
            .unwrap()
            .pop()
            .unwrap_or_else(|| "@done".into()))
    }
}

/// What the session's surface does: `sys:todo` writes the task's list (when
/// it keeps one), and every read answers [`file`].
pub(super) struct Surface(pub(super) Option<Checklist>);

impl ToolRunner for Surface {
    fn hint(&self) -> String {
        "TOOLS: sys:todo, sys:read_file".into()
    }
    fn call(&self, _server: &str, tool: &str, args: &str) -> Result<String, String> {
        match (tool, &self.0) {
            ("todo", Some(list)) => list.write(args),
            _ => Ok(file(args)),
        }
    }
    fn checklist(&self) -> Option<&Checklist> {
        self.0.as_ref()
    }
}

/// 2,470 chars: well over the 600 that are never shortened.
pub(super) fn file(args: &str) -> String {
    (0..40)
        .map(|i| format!("{args} line {i:02}: {}", "x".repeat(30)))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn todo(said: &str, items: &[(&str, &str)]) -> String {
    let items: Vec<serde_json::Value> = items
        .iter()
        .map(|(text, status)| serde_json::json!({"text": text, "status": status}))
        .collect();
    format!(
        "{said}\n@tool sys:todo {}",
        serde_json::json!({ "items": items })
    )
}

pub(super) fn read(name: &str) -> String {
    format!("reading {name}\n@tool sys:read_file {{\"path\":\"{name}\"}}")
}

/// Run one turn of `replies` against `surface`: every prompt the agent was
/// dialed with, and the thread's final answer.
pub(super) fn turn(surface: Surface, replies: Vec<String>) -> (Vec<String>, String) {
    let dialed = Arc::new(Mutex::new(Vec::new()));
    let mut replies = replies;
    replies.reverse();
    let agent: Box<dyn Adapter> = Box::new(Scripted(Mutex::new(replies), Arc::clone(&dialed)));
    let b = Broker::new(Registry::new(vec![agent]), 2, Duration::from_secs(5))
        .with_tools(Arc::new(surface))
        .with_tool_rounds(8);
    let mut answer = String::new();
    b.run(
        "user",
        "coder",
        "add a /foo command with tests and docs",
        "t1",
        &crate::broker::tick::noop_tick_emit(),
        &mut |hop| {
            if hop.kind == HopKind::Done {
                answer = hop.text;
            }
        },
    );
    let dialed = dialed.lock().unwrap().clone();
    (dialed, answer)
}
