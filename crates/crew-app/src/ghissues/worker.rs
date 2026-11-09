//! The sync's thread: a pass at once, then one when the todo list has been
//! still a moment after a change, and every few minutes for what changed on
//! GitHub. A pass asks GitHub about the live issues, plans
//! ([`super::plan`]), writes GitHub first and crew after, each crew write
//! only over a todo nobody touched while the pass ran.
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use super::plan::{self, Step};
use super::repo::Repos;
use super::{gh, Log};
use crate::applog::LogLevel;
use crate::todopane::item::TodoIssue;
use crate::todopane::store;

const EVERY: Duration = Duration::from_secs(300);
const SETTLE: Duration = Duration::from_secs(3);

pub(super) fn run(log: &Log, stop: &AtomicBool) {
    let mut said = HashSet::new();
    let mut last = Instant::now() - EVERY;
    let mut seen = 0;
    let mut moved_at: Option<Instant> = None;
    while !stop.load(Ordering::Relaxed) {
        let rev = store::revision();
        if rev != seen {
            moved_at = Some(Instant::now());
            seen = rev;
        }
        let settled = moved_at.is_some_and(|t| t.elapsed() >= SETTLE);
        if settled || last.elapsed() >= EVERY {
            pass(log, &mut said);
            seen = store::revision();
            moved_at = None;
            last = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// Say `msg` once per run, not every pass.
fn once(log: &Log, said: &mut HashSet<String>, level: LogLevel, msg: String) {
    if said.insert(msg.clone()) {
        let _ = log.send((level, msg));
    }
}

/// One pass. A failure costs only the issue it is about (one deleted on
/// GitHub, a repo with issues turned off) and is said once; the rest of the
/// list still goes across, and the next pass tries that one again.
fn pass(log: &Log, said: &mut HashSet<String>) {
    let items = store::snapshot();
    let mut remote = HashMap::new();
    for it in items.iter().filter(|it| plan::live(it)) {
        let Some(i) = &it.issue else { continue };
        match gh::view(&i.repo, i.number) {
            Ok(r) => drop(remote.insert(it.id, r)),
            Err(e) => once(log, said, LogLevel::Error, format!("GitHub issues: {e}")),
        }
    }
    let mut repos = Repos::default();
    for step in plan::plan(&items, &mut |p| repos.of(p), &remote) {
        if let Err(e) = apply(step, log) {
            once(log, said, LogLevel::Error, format!("GitHub issues: {e}"));
        }
    }
}

fn body(id: u64) -> String {
    format!(
        "Filed from crew's /todo list (todo {id}).\n\n\
         Closing this issue ticks the todo, and ticking the todo closes it. \
         From any shell: `crew todo done {id}`."
    )
}

fn apply(step: Step, log: &Log) -> Result<(), String> {
    match step {
        Step::File { todo, repo, title } => {
            let (number, url) = gh::create(&repo, &title, &body(todo))?;
            let _ = log.send((
                LogLevel::Info,
                format!("GitHub issues: filed {repo}#{number} \u{2014} {title}"),
            ));
            let issue = TodoIssue {
                repo,
                number,
                url,
                title,
                open: true,
            };
            store::mutate(|items| {
                if let Some(it) = items
                    .iter_mut()
                    .find(|it| it.id == todo && it.issue.is_none())
                {
                    it.issue = Some(issue);
                }
            });
        }
        Step::Sync {
            todo,
            issue,
            was,
            title,
            open,
            push_title,
            push_open,
        } => {
            if push_title {
                gh::retitle(&issue.repo, issue.number, &title)?;
            }
            if push_open {
                gh::set_open(&issue.repo, issue.number, open)?;
            }
            store::mutate(|items| {
                let it = items.iter_mut().find(|it| it.id == todo);
                let Some(it) = it.filter(|it| (it.title.clone(), it.done) == was) else {
                    return;
                };
                if it.done == open {
                    it.done = !open;
                    it.done_ms = (!open).then(crate::chattime::unix_now_ms);
                }
                it.title = title.clone();
                it.issue = Some(TodoIssue {
                    title,
                    open,
                    ..issue
                });
            });
        }
    }
    Ok(())
}
