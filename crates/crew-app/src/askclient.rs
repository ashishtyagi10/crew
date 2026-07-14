//! Client side of inter-pane `ask`: the `crew ask <to> "<q>"` and `crew panes`
//! subcommands. They connect to the running GUI's IPC socket, send one
//! request, print the reply, and exit — short-circuited in `main.rs` before
//! any GUI init (like `--list-fonts`). All the waiting happens app-side.
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

use crate::askrender::render;
use crate::ipc_types::{CastMode, Reply, Request, PROTOCOL_V};

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
