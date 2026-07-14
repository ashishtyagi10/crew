//! Client side of inter-pane `ask`: the `crew ask <to> "<q>"` and `crew panes`
//! subcommands. They connect to the running GUI's IPC socket, send one
//! request, print the reply, and exit — short-circuited in `main.rs` before
//! any GUI init (like `--list-fonts`). All the waiting happens app-side.
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

use crate::ipc_types::{CastAnswer, CastMode, NoAnswer, PaneCard, Reply, Request, PROTOCOL_V};

/// Render a reply to (human text, process exit code): 0 answered/roster,
/// 2 no-answer, 3 unreachable/no-crew.
pub(crate) fn render(r: &Reply) -> (String, i32) {
    match r {
        Reply::Answered { text } => (format!("ANSWERED: {text}"), 0),
        Reply::NoAnswer { reason, partial } => {
            let why = match reason {
                NoAnswer::IdleNoEngage => "idle (target never engaged)",
                NoAnswer::Stalled => "stalled (target stopped without finishing)",
                NoAnswer::BusyElsewhere => "busy (target is working on its own task)",
                NoAnswer::Unreachable => "unreachable (no such pane)",
            };
            let mut s = format!("NO_ANSWER: {why}");
            if let Some(p) = partial.as_deref().filter(|p| !p.is_empty()) {
                s.push_str(&format!("\n--- partial ---\n{p}"));
            }
            let code = if matches!(reason, NoAnswer::Unreachable) {
                3
            } else {
                2
            };
            (s, code)
        }
        Reply::Roster { panes } => (render_roster(panes), 0),
        Reply::Cast { answers } => render_cast(answers),
    }
}

/// Render a broadcast reply to (text, exit code): 0 if anyone answered, 2 if
/// panes were reached but none answered, 3 if no pane was eligible.
fn render_cast(answers: &[CastAnswer]) -> (String, i32) {
    if answers.is_empty() {
        return (
            "NO_ANSWER: no eligible panes to broadcast to".to_string(),
            3,
        );
    }
    let mut out = String::new();
    let mut answered = 0;
    for a in answers {
        let who = a.label.as_deref().unwrap_or(&a.pane);
        match (&a.text, a.no_answer) {
            (Some(t), _) => {
                answered += 1;
                out.push_str(&format!("[{who}] ANSWERED: {t}\n"));
            }
            (None, Some(NoAnswer::Stalled)) => {
                out.push_str(&format!("[{who}] no answer (stalled)\n"))
            }
            (None, _) => out.push_str(&format!("[{who}] no answer (idle)\n")),
        }
    }
    (out.trim_end().to_string(), if answered > 0 { 0 } else { 2 })
}

fn render_roster(panes: &[PaneCard]) -> String {
    let mut out = String::from("id   label            kind      running   state\n");
    for c in panes {
        out.push_str(&format!(
            "{:<4} {:<16} {:<9} {:<9} {}\n",
            c.id,
            c.label.as_deref().unwrap_or("-"),
            c.kind,
            c.running.as_deref().unwrap_or("-"),
            if c.busy { "busy" } else { "idle" },
        ));
    }
    out.trim_end().to_string()
}

/// Send one request over the socket and read the reply. `instance` selects
/// which crew to dial (`None` = this instance's own socket, for a local ask;
/// `Some(id)` = a federated `pane@instance` target). `None` return if no crew
/// is listening (socket absent / refused).
fn exchange(req: &Request, instance: Option<&str>) -> Option<Reply> {
    let path = match instance {
        Some(id) => crate::ipc::socket_path_for(Some(id)),
        None => crate::ipc::socket_path(),
    };
    let mut stream = UnixStream::connect(path).ok()?;
    let json = serde_json::to_string(req).ok()?;
    stream.write_all(json.as_bytes()).ok()?;
    stream.write_all(b"\n").ok()?;
    stream.flush().ok()?;
    let mut line = String::new();
    BufReader::new(&mut stream).read_line(&mut line).ok()?;
    serde_json::from_str(line.trim()).ok()
}

/// The unreachable/no-crew message + exit code, shared by both subcommands.
fn no_crew() -> (String, i32) {
    ("NO_ANSWER: unreachable (no crew running)".to_string(), 3)
}

/// `crew ask <to> "<question>"`. `<to>` may be `pane@instance` to reach an
/// agent in another crew instance's pane (v3 federation) — the instance part
/// picks the socket, the pane part is resolved by that crew unchanged.
pub(crate) fn run_ask(to: &str, question: &str) -> i32 {
    let (pane, instance) = match crate::askaddr::resolve_target(to) {
        Ok(v) => v,
        Err(msg) => {
            println!("NO_ANSWER: {msg}");
            return 3;
        }
    };
    let from = std::env::var("CREW_PANE").unwrap_or_else(|_| "an agent".to_string());
    let id = format!("q{}", std::process::id());
    let req = Request::Ask {
        v: PROTOCOL_V,
        from,
        to: pane.to_string(),
        question: question.to_string(),
        id,
    };
    let (text, code) = exchange(&req, instance.as_deref())
        .map(|r| render(&r))
        .unwrap_or_else(no_crew);
    println!("{text}");
    code
}

/// `crew ask --all|--any "<question>"` — fan one question across every eligible
/// pane and print the aggregate.
pub(crate) fn run_broadcast(mode: CastMode, question: &str) -> i32 {
    let from = std::env::var("CREW_PANE").unwrap_or_else(|_| "an agent".to_string());
    let id = format!("b{}", std::process::id());
    let req = Request::Broadcast {
        v: PROTOCOL_V,
        from,
        question: question.to_string(),
        id,
        mode,
    };
    let (text, code) = exchange(&req, None)
        .map(|r| render(&r))
        .unwrap_or_else(no_crew);
    println!("{text}");
    code
}

/// `crew panes`.
pub(crate) fn run_panes() -> i32 {
    let (text, code) = exchange(&Request::Panes { v: PROTOCOL_V }, None)
        .map(|r| render(&r))
        .unwrap_or_else(no_crew);
    println!("{text}");
    code
}

/// `crew instances` — list crew instances discoverable by their local sockets
/// (opt-in discovery; `pane@<id>` addresses one). "default" is the unnamed
/// instance.
pub(crate) fn run_instances() -> i32 {
    let mut ids = crate::ipc::list_instances();
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        println!("(no crew instances running)");
    } else {
        for id in ids {
            println!("{id}");
        }
    }
    0
}

/// Route a `crew ask …` / `crew panes` client subcommand. `Some(exit code)`
/// when the args were a client subcommand (the caller exits without launching
/// the GUI); `None` when they are not ours and startup should continue.
pub(crate) fn dispatch_cli() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("ask") if args.len() >= 3 && (args[1] == "--all" || args[1] == "--any") => {
            let mode = if args[1] == "--any" {
                CastMode::Any
            } else {
                CastMode::All
            };
            Some(run_broadcast(mode, &args[2]))
        }
        Some("ask") if args.len() >= 3 => Some(run_ask(&args[1], &args[2])),
        Some("ask") => {
            eprintln!(
                "usage: crew ask <pane-id-or-label> \"<question>\"\n\
                        crew ask --all|--any \"<question>\"   (broadcast)"
            );
            Some(64)
        }
        Some("panes") => Some(run_panes()),
        Some("instances") => Some(run_instances()),
        _ => None,
    }
}

#[cfg(test)]
#[path = "askclient_tests.rs"]
mod tests;
