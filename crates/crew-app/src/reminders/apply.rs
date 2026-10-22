//! The crew half of a plan: its todo steps written into the list, each only
//! over the todo it was planned from, so an edit made while the pass ran is
//! never written over.
use super::fields::Fields;
use super::plan::{Pair, Step};
use crate::todopane::item::TodoItem;

/// `steps`' todo half applied to `items`; the pairs its new todos make.
pub(crate) fn todos(items: &mut Vec<TodoItem>, steps: &[Step], now_ms: u64) -> Vec<Pair> {
    let mut born = Vec::new();
    for s in steps {
        match s {
            Step::SetTodo { todo, was, f } => {
                let it = items.iter_mut().find(|it| it.id == *todo);
                if let Some(it) = it.filter(|it| Fields::of(it) == *was) {
                    set(it, f, now_ms);
                }
            }
            Step::DropTodo { todo, was } => {
                items.retain(|it| it.id != *todo || Fields::of(it) != *was);
            }
            Step::NewTodo { rem, f } => {
                // As the composer numbers a new todo (`act`).
                let next = items.iter().map(|i| i.id).max().map_or(0, |m| m + 1);
                let mut it = TodoItem {
                    id: now_ms.max(next),
                    created_ms: now_ms,
                    ..TodoItem::default()
                };
                set(&mut it, f, now_ms);
                born.push(Pair {
                    todo: it.id,
                    rem: rem.clone(),
                    last: f.clone(),
                });
                items.push(it);
            }
            _ => {}
        }
    }
    born
}

/// One todo to say `f`. A tick takes crew's own stamp of when; a due that
/// moved may toast again, as an edit's does, unless it is already past.
fn set(it: &mut TodoItem, f: &Fields, now_ms: u64) {
    it.title.clone_from(&f.title);
    if it.done != f.done {
        it.done = f.done;
        it.done_ms = f.done.then_some(now_ms);
    }
    if Fields::of(it).due != f.due {
        let due = f.due_ms();
        it.due_ms = due.map(|d| d.0);
        it.due_has_time = due.is_some_and(|d| d.1);
        it.notified = it.due_ms.is_some_and(|d| d <= now_ms);
    }
}

#[cfg(test)]
#[path = "apply_tests.rs"]
mod tests;
