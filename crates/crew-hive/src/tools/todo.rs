//! A checklist an agent keeps for its own task (`sys:todo`), after Claude
//! Code's TodoWrite.
//!
//! An agent asked to "add a /foo command with tests and docs" planned the
//! steps in its head and lost them as the rounds went by: older results are
//! shortened (`exchanges`), the reply that listed the steps sinks under later
//! ones, and a turn that had done two of three said done. Now it can write the
//! steps down. The list rides at the top of every follow-up prompt of the
//! task, above the exchanges, where no shortening reaches it, and a task that
//! ends with steps left says which under its answer.
//!
//! The model sends the WHOLE list each time, and it replaces the last one.
//! Edits to a list (tick item 2, add one after it) would need ids the model
//! has to keep straight, and a wrong one is a list that drifts from what the
//! model believes without either of them noticing; a whole list is always
//! what it believes.
//!
//! One list per task: the relay's lives on the session's task snapshot, and a
//! swarm worker's on a surface of its own ([`per_task`]), so workers running
//! at once never write over each other's.

use std::sync::{Arc, Mutex, MutexGuard};

use serde_json::Value;

use super::ToolCall;

#[path = "todotask.rs"]
mod task;
pub use task::per_task;

/// Items a list may hold. Past a dozen it is a plan rather than a checklist,
/// and every item rides in every follow-up prompt of the task.
pub const MAX_ITEMS: usize = 12;

/// Chars an item may run to: a step, not a paragraph.
pub const MAX_TEXT: usize = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Pending,
    InProgress,
    Done,
}

impl Status {
    /// Item `n`'s status as sent. Absent means not started. TodoWrite's
    /// `completed` is read as `done`: a model that learned that tool reaches
    /// for its word, and refusing it would cost a round to say the same thing.
    fn read(v: Option<&Value>, n: usize) -> Result<Self, String> {
        let Some(v) = v else {
            return Ok(Self::Pending);
        };
        match v.as_str() {
            Some("pending") => Ok(Self::Pending),
            Some("in_progress") => Ok(Self::InProgress),
            Some("done" | "completed") => Ok(Self::Done),
            _ => Err(format!(
                "item {n} has status {v} \u{2014} use pending, in_progress or done"
            )),
        }
    }

    fn mark(self) -> char {
        match self {
            Self::Pending => '\u{2610}',
            Self::InProgress => '\u{25b6}',
            Self::Done => '\u{2611}',
        }
    }
}

#[derive(Clone, Debug)]
struct Item {
    text: String,
    status: Status,
}

/// One task's checklist, and how many times it has been written.
#[derive(Default, Debug)]
pub struct Checklist(Mutex<List>);

#[derive(Default, Debug)]
struct List {
    items: Vec<Item>,
    writes: u32,
}

impl Checklist {
    /// A task's list, with nothing on it.
    pub fn new() -> Arc<Self> {
        Arc::default()
    }

    /// `sys:todo`: `args` holds the whole new list, which replaces this one
    /// when it is valid; the answer is the list as the prompts will show it.
    /// An invalid list is refused whole and the old one stands.
    pub fn write(&self, args: &str) -> Result<String, String> {
        let items = parse(args)?;
        let shown = render(&items);
        let mut list = self.lock();
        list.items = items;
        list.writes += 1;
        Ok(shown)
    }

    /// How many writes the list has taken, so a loop can tell a round that
    /// changed it from one that did not.
    pub fn writes(&self) -> u32 {
        self.lock().writes
    }

    /// The list as it heads a follow-up prompt, with the blank line after it,
    /// or nothing at all while the list is empty, so a task that never kept
    /// one is sent exactly what it was sent before there were lists.
    pub fn section(&self) -> String {
        let list = self.lock();
        match list.items.is_empty() {
            true => String::new(),
            false => format!("YOUR CHECKLIST:\n{}\n\n", render(&list.items)),
        }
    }

    /// `checklist: 3 of 5 done — not done: write the docs, add a test`, or
    /// `None` when nothing on the list is left.
    pub fn unfinished(&self) -> Option<String> {
        let list = self.lock();
        let left: Vec<&str> = list
            .items
            .iter()
            .filter(|i| i.status != Status::Done)
            .map(|i| i.text.as_str())
            .collect();
        (!left.is_empty()).then(|| {
            format!(
                "checklist: {} of {} done \u{2014} not done: {}",
                list.items.len() - left.len(),
                list.items.len(),
                left.join(", ")
            )
        })
    }

    fn lock(&self) -> MutexGuard<'_, List> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Whether `call` writes the checklist.
pub fn is_todo(call: &ToolCall) -> bool {
    (call.server.as_str(), call.tool.as_str()) == ("sys", "todo")
}

/// `list`'s section for the top of a follow-up prompt; nothing without one.
pub fn section(list: Option<&Checklist>) -> String {
    list.map(Checklist::section).unwrap_or_default()
}

/// A task's final answer with the line naming what its list left undone
/// under it, so a skipped step is not hidden behind an answer that says done.
/// Unchanged when there is no list or nothing on it is left.
pub fn finished(answer: String, list: Option<&Checklist>) -> String {
    let Some(line) = list.and_then(Checklist::unfinished) else {
        return answer;
    };
    match answer.trim_end() {
        "" => line,
        kept => format!("{kept}\n\n{line}"),
    }
}

fn parse(args: &str) -> Result<Vec<Item>, String> {
    let v: Value = serde_json::from_str(args.trim())
        .map_err(|e| format!("arguments are not valid JSON: {e}"))?;
    let items = v.get("items").and_then(Value::as_array).ok_or(
        "missing array argument \u{201c}items\u{201d} \u{2014} send the whole list: \
         {\"items\": [{\"text\": \u{2026}, \"status\": \"pending\"}]}",
    )?;
    if items.len() > MAX_ITEMS {
        return Err(format!(
            "{} items \u{2014} a checklist holds at most {MAX_ITEMS}; fold small steps together",
            items.len()
        ));
    }
    let items: Vec<Item> = items
        .iter()
        .enumerate()
        .map(|(i, v)| item(v, i + 1))
        .collect::<Result<_, _>>()?;
    let doing: Vec<String> = (1..=items.len())
        .filter(|n| items[n - 1].status == Status::InProgress)
        .map(|n| n.to_string())
        .collect();
    if doing.len() > 1 {
        return Err(format!(
            "items {} are all in_progress \u{2014} at most ONE item may be in_progress: \
             the one you are on now",
            doing.join(", ")
        ));
    }
    Ok(items)
}

/// Item `n`, its text on one line. `content` is TodoWrite's name for the text
/// and is read for the same reason `completed` is.
fn item(v: &Value, n: usize) -> Result<Item, String> {
    let raw = v.get("text").or_else(|| v.get("content"));
    let text = raw.and_then(Value::as_str).unwrap_or_default();
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return Err(format!("item {n} has no \u{201c}text\u{201d}"));
    }
    let len = text.chars().count();
    if len > MAX_TEXT {
        return Err(format!(
            "item {n} is {len} chars \u{2014} keep each to {MAX_TEXT}"
        ));
    }
    let status = Status::read(v.get("status"), n)?;
    Ok(Item { text, status })
}

/// `☐ text` / `▶ text` / `☑ text`, one a line, then `2 of 5 done`.
fn render(items: &[Item]) -> String {
    if items.is_empty() {
        return "checklist cleared".into();
    }
    let done = items.iter().filter(|i| i.status == Status::Done).count();
    let mut lines: Vec<String> = items
        .iter()
        .map(|i| format!("{} {}", i.status.mark(), i.text))
        .collect();
    lines.push(format!("{done} of {} done", items.len()));
    lines.join("\n")
}

#[cfg(test)]
#[path = "todo_tests.rs"]
mod tests;
