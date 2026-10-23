//! The sync's one decision, made with no Reminders and no disk in sight:
//! given the todos, the reminders in the `crew` list and the pairs agreed
//! last time, what to write where.
//!
//! A pair remembers what both sides said when they last agreed. A side whose
//! field differs from that has changed, and the change goes to the other
//! side; when both changed one field, crew's word stands. A side that lost
//! its half was deleted there, and the delete goes across, unless the other
//! half changed since (that edit wins, and the thread comes back), or it is a
//! done todo whose reminder Reminders cleared away (crew keeps its history).
//! A todo or reminder with no pair finds one by title first, so turning the
//! sync on twice never makes two of anything — but only one that is as done
//! as it is: last month's done "pay rent" is history, and this month's new
//! one on the phone is news, not a tick for the old one. Failing that, an
//! open one is made on the other side and a done one is left alone.
//!
//! Neither side loses its list to the other answering empty: when every
//! paired reminder is gone at once, the pairs are forgotten and the open
//! todos go to Reminders again; when every paired TODO is gone at once (a
//! `todos.toml` that would not load reads as no todos), the open reminders
//! come back as todos, rather than all of them being deleted.
use serde::{Deserialize, Serialize};

use super::fields::Fields;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct Pair {
    pub todo: u64,
    pub rem: String,
    /// What both sides said when they last agreed.
    pub last: Fields,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Step {
    /// A reminder for this todo, in the `crew` list.
    NewRem {
        todo: u64,
        f: Fields,
    },
    /// The reminder, saying `was` now, to say `f`.
    SetRem {
        rem: String,
        was: Fields,
        f: Fields,
    },
    DropRem {
        rem: String,
    },
    /// A todo for this reminder.
    NewTodo {
        rem: String,
        f: Fields,
    },
    /// The todo to say `f`, if it still says `was`: an edit made while the
    /// sync ran is not written over, and the next pass sees it.
    SetTodo {
        todo: u64,
        was: Fields,
        f: Fields,
    },
    DropTodo {
        todo: u64,
        was: Fields,
    },
}

#[derive(Debug, Default)]
pub(crate) struct Plan {
    pub steps: Vec<Step>,
    /// The pairs that stand after the steps, but for those `NewRem` and
    /// `NewTodo` make, which need the id the write gives.
    pub pairs: Vec<Pair>,
}

pub(crate) fn plan(todos: &[(u64, Fields)], rems: &[(String, Fields)], pairs: &[Pair]) -> Plan {
    let rems_gone = pairs.iter().all(|p| !rems.iter().any(|r| r.0 == p.rem));
    let todos_gone = pairs.iter().all(|p| !todos.iter().any(|t| t.0 == p.todo));
    let pairs = match pairs.len() >= 2 && (rems_gone || todos_gone) {
        true => &[][..],
        false => pairs,
    };
    let mut free: Vec<&(String, Fields)> = (rems.iter())
        .filter(|r| !pairs.iter().any(|p| p.rem == r.0))
        .collect();
    let mut out = Plan::default();
    for p in pairs {
        let t = todos.iter().find(|t| t.0 == p.todo);
        // A reminder whose id changed under it still has its title.
        let r = rems.iter().find(|r| r.0 == p.rem).or_else(|| {
            let i = free.iter().position(|r| r.1.same_title(&p.last))?;
            Some(free.remove(i))
        });
        match (t, r) {
            (Some(t), Some(r)) => {
                let f = merged(&p.last, &t.1, &r.1);
                out.agree(t, r, f);
            }
            (Some(t), None) if t.1.done => {} // cleared there; kept here
            (Some(t), None) if t.1 != p.last => out.steps.push(Step::NewRem {
                todo: t.0,
                f: t.1.clone(),
            }),
            (Some(t), None) => out.steps.push(Step::DropTodo {
                todo: t.0,
                was: t.1.clone(),
            }),
            (None, Some(r)) if r.1 != p.last && !r.1.done => out.steps.push(Step::NewTodo {
                rem: r.0.clone(),
                f: r.1.clone(),
            }),
            (None, Some(r)) => out.steps.push(Step::DropRem { rem: r.0.clone() }),
            (None, None) => {}
        }
    }
    let paired = |id: u64| pairs.iter().any(|p| p.todo == id);
    for t in todos.iter().filter(|t| !paired(t.0)) {
        match (free.iter()).position(|r| r.1.same_title(&t.1) && r.1.done == t.1.done) {
            Some(i) => {
                let r = free.remove(i);
                let f = Fields {
                    title: t.1.title.clone(),
                    done: t.1.done,
                    due: t.1.due.clone().or_else(|| r.1.due.clone()),
                };
                out.agree(t, r, f);
            }
            None if !t.1.done => out.steps.push(Step::NewRem {
                todo: t.0,
                f: t.1.clone(),
            }),
            None => {}
        }
    }
    for r in free.into_iter().filter(|r| !r.1.done) {
        out.steps.push(Step::NewTodo {
            rem: r.0.clone(),
            f: r.1.clone(),
        });
    }
    out
}

impl Plan {
    /// Both halves of a pair brought to `f`, and the pair kept.
    fn agree(&mut self, t: &(u64, Fields), r: &(String, Fields), f: Fields) {
        if t.1 != f {
            self.steps.push(Step::SetTodo {
                todo: t.0,
                was: t.1.clone(),
                f: f.clone(),
            });
        }
        if r.1 != f {
            self.steps.push(Step::SetRem {
                rem: r.0.clone(),
                was: r.1.clone(),
                f: f.clone(),
            });
        }
        self.pairs.push(Pair {
            todo: t.0,
            rem: r.0.clone(),
            last: f,
        });
    }
}

/// Field by field: what changed since `last` wins, crew's change first.
fn merged(last: &Fields, crew: &Fields, rem: &Fields) -> Fields {
    let pick = |c: bool| if c { crew } else { rem };
    Fields {
        title: pick(crew.title != last.title).title.clone(),
        done: pick(crew.done != last.done).done,
        due: pick(crew.due != last.due).due.clone(),
    }
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "plan_title_tests.rs"]
mod title_tests;
