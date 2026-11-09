//! `crew todo` — the /todo list from any shell, so an agent in another pane
//! (Claude Code, Codex, smith through `sys:run`) can read the list, file a
//! todo and tick one off. It reads and writes the same `todos.toml` the app
//! does; a running crew sees the change within a second (`store`'s watch),
//! and the GitHub sync carries it to the todo's issue.
use crate::todopane::item::TodoItem;
use crate::todopane::{duetext, store};

pub(crate) const USAGE: &str = "\
usage: crew todo [list] [--all] [@project] [#who] [--json]   open todos (--all adds done)
       crew todo add <text>        e.g. crew todo add \"fix the build fri @crew #ana\"
       crew todo done <id>         tick a todo (any unique tail of its id)
       crew todo undone <id>       open it again";

/// Run `crew todo <args>`; the process exit code.
pub(crate) fn run(args: &[String]) -> i32 {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        [] => list(&[]),
        ["list" | "ls", rest @ ..] => list(rest),
        ["add", text @ ..] if !text.is_empty() => add(&text.join(" ")),
        [verb @ ("done" | "undone"), id] => tick(id, *verb == "done"),
        ["-h" | "--help" | "help", ..] => {
            println!("{USAGE}");
            0
        }
        [first, ..] if first.starts_with(['@', '#', '-']) => list(&words),
        _ => {
            eprintln!("{USAGE}");
            64
        }
    }
}

fn list(flags: &[&str]) -> i32 {
    let all = flags.contains(&"--all");
    let json = flags.contains(&"--json");
    let tag = |sigil: char| {
        flags
            .iter()
            .find_map(|f| f.strip_prefix(sigil))
            .filter(|t| !t.is_empty())
    };
    let (project, who) = (tag('@'), tag('#'));
    let items = store::snapshot();
    let f = crate::todopane::item::Filters { project, who };
    let mut order = crate::todopane::item::display_order(&items, f, all);
    if all {
        order.sort_by_key(|&i| items[i].done);
    }
    let shown: Vec<&TodoItem> = order.iter().map(|&i| &items[i]).collect();
    if json {
        return match serde_json::to_string_pretty(&shown) {
            Ok(s) => {
                println!("{s}");
                0
            }
            Err(e) => fail(&e.to_string()),
        };
    }
    if shown.is_empty() {
        println!("no todos");
    }
    let now = crate::chattime::unix_now_ms();
    for it in shown {
        println!("{}", line(it, now));
    }
    0
}

/// One todo as a line: box, id, title, tags, due, issue.
pub(crate) fn line(it: &TodoItem, now_ms: u64) -> String {
    let mut s = format!(
        "{} {}  {}",
        if it.done { "[x]" } else { "[ ]" },
        it.id,
        it.title
    );
    if let Some(p) = &it.project {
        s += &format!("  @{p}");
    }
    if let Some(w) = &it.assignee {
        s += &format!("  #{w}");
    }
    if let Some(d) = it.due_ms {
        s += &format!("  due {}", duetext::label(d, it.due_has_time, now_ms));
    }
    if let Some(i) = &it.issue {
        s += &format!("  {}#{}", i.repo, i.number);
    }
    s
}

fn add(text: &str) -> i32 {
    let Some(e) = crate::todoentry::parse(text) else {
        return fail("a due date or a tag alone is not a todo");
    };
    let now = crate::chattime::unix_now_ms();
    let it = store::mutate(|items| {
        let id = now.max(items.iter().map(|i| i.id).max().map_or(0, |m| m + 1));
        let it = TodoItem {
            id,
            title: e.title,
            project: e.project,
            assignee: e.assignee,
            notified: e.due_ms.is_some_and(|d| d <= now),
            due_ms: e.due_ms,
            due_has_time: e.has_time,
            created_ms: now,
            ..TodoItem::default()
        };
        items.push(it.clone());
        it
    });
    println!("added {}", line(&it, now));
    0
}

fn tick(id: &str, done: bool) -> i32 {
    let now = crate::chattime::unix_now_ms();
    let r: Result<TodoItem, String> = store::mutate(|items| {
        let i = find(items, id)?;
        let it = &mut items[i];
        if it.done != done {
            it.done = done;
            it.done_ms = done.then_some(now);
        }
        Ok(it.clone())
    });
    match r {
        Ok(it) => {
            println!("{}", line(&it, now));
            0
        }
        Err(e) => fail(&e),
    }
}

/// The one todo whose id is `id` or ends in it.
pub(crate) fn find(items: &[TodoItem], id: &str) -> Result<usize, String> {
    let id = id.trim_start_matches('#');
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            "{id:?} is not a todo id (crew todo list shows them)"
        ));
    }
    let hits: Vec<usize> = (0..items.len())
        .filter(|&i| items[i].id.to_string().ends_with(id))
        .collect();
    match hits.as_slice() {
        [i] => Ok(*i),
        [] => Err(format!("no todo has id {id}")),
        _ => Err(format!(
            "{} todos end in {id}: give more digits",
            hits.len()
        )),
    }
}

fn fail(msg: &str) -> i32 {
    eprintln!("crew todo: {msg}");
    1
}

#[cfg(test)]
#[path = "todocli_tests.rs"]
mod tests;
