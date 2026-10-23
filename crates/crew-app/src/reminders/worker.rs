//! The sync's thread: a pass at once, then one whenever the todo list has
//! been still a moment after a change, Reminders says it moved, or a minute
//! goes by. It says what came from Reminders, and what went wrong once
//! rather than every minute.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use super::ek::Kit;
use super::fields::Fields;
use super::plan::{self, Pair, Step};
use super::{apply, pairs, Log};
use crate::applog::LogLevel;
use crate::todopane::store;

const EVERY: Duration = Duration::from_secs(60);
/// How long the todo list stays still after a change before it goes across:
/// a few quick edits make one pass.
const SETTLE: Duration = Duration::from_millis(1500);

pub(super) fn run(log: &Log, stop: &AtomicBool) {
    let kit = match Kit::open() {
        Ok(k) => k,
        Err(e) => return drop(log.send((LogLevel::Error, e))),
    };
    let (tx, rx) = mpsc::channel();
    let watch = kit.watch(tx);
    let mut said = String::new();
    let mut first = true;
    while !stop.load(Ordering::Relaxed) {
        let line = match pass(&kit) {
            Ok(_) if first => Some((
                LogLevel::Info,
                "Reminders: /todo and the crew list are in step".into(),
            )),
            Ok(_) if !said.is_empty() => Some((LogLevel::Info, "Reminders: in step again".into())),
            Ok(0) => None,
            Ok(n) => Some((
                LogLevel::Info,
                format!(
                    "Reminders: {n} change{} from Reminders",
                    if n == 1 { "" } else { "s" }
                ),
            )),
            Err(e) if e == said => None,
            Err(e) => Some((LogLevel::Error, e)),
        };
        if let Some((level, text)) = line {
            said = if level == LogLevel::Error {
                text.clone()
            } else {
                String::new()
            };
            let _ = log.send((level, text));
        }
        first = false;
        wait(&rx, stop);
    }
    Kit::unwatch(&watch);
}

/// Until the next pass is due.
fn wait(rx: &Receiver<()>, stop: &AtomicBool) {
    let start = Instant::now();
    let mut seen = store::revision();
    let mut moved: Option<Instant> = None;
    while !stop.load(Ordering::Relaxed) && start.elapsed() < EVERY {
        if rx.recv_timeout(Duration::from_millis(250)).is_ok() {
            while rx.try_recv().is_ok() {}
            return;
        }
        let rev = store::revision();
        if rev != seen {
            (seen, moved) = (rev, Some(Instant::now()));
        }
        if moved.is_some_and(|m| m.elapsed() >= SETTLE) {
            return;
        }
    }
}

/// One pass: both sides read, the plan written, Reminders first so a write
/// that fails leaves crew and the pairs as they were. How many todos
/// Reminders changed.
fn pass(kit: &Kit) -> Result<usize, String> {
    let file = pairs::path();
    let mut state = pairs::load_at(file.as_deref());
    let (list, other) = kit.list(&state.list, state.remake)?;
    // Asked once, used once: a list deleted later stays deleted.
    let asked = std::mem::take(&mut state.remake);
    if other {
        // Not the list the pairs were made in: pair again, by title.
        state = pairs::State {
            list: Kit::id(&list),
            ..Default::default()
        };
    }
    let rems = kit.fetch(&list)?;
    let todos: Vec<(u64, Fields)> = (store::snapshot().iter())
        .map(|it| (it.id, Fields::of(it)))
        .collect();
    if store::take_unreadable() {
        // todos.toml would not load: no todos here is not every todo
        // deleted, so the open reminders come back by title.
        state.pairs.clear();
    }
    let plan = plan::plan(&todos, &rems, &state.pairs);
    let mut made = Vec::new();
    let wrote = (|| {
        for s in &plan.steps {
            match s {
                Step::NewRem { todo, f } => made.push(Pair {
                    todo: *todo,
                    rem: kit.put(&list, None, None, f)?,
                    last: f.clone(),
                }),
                Step::SetRem { rem, was, f } => drop(kit.put(&list, Some(rem), Some(was), f)?),
                Step::DropRem { rem } => kit.remove(rem)?,
                _ => {}
            }
        }
        kit.commit()
    })();
    if let Err(e) = wrote {
        kit.rollback();
        return Err(e);
    }
    let here = |s: &&Step| {
        matches!(
            s,
            Step::NewTodo { .. } | Step::SetTodo { .. } | Step::DropTodo { .. }
        )
    };
    let came = plan.steps.iter().filter(here).count();
    let born = match came {
        0 => Vec::new(),
        _ => {
            store::mutate(|items| apply::todos(items, &plan.steps, crate::chattime::unix_now_ms()))
        }
    };
    let agreed: Vec<Pair> = plan.pairs.into_iter().chain(made).chain(born).collect();
    if other || asked || agreed != state.pairs {
        state.pairs = agreed;
        pairs::save_at(file.as_deref(), &state);
    }
    Ok(came)
}
