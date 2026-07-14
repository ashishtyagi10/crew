//! Client side of inter-pane `ask`: the `crew ask <to> "<q>"` and `crew panes`
//! subcommands. They connect to the running GUI's IPC socket, send one
//! request, print the reply, and exit — short-circuited in `main.rs` before
//! any GUI init (like `--list-fonts`). All the waiting happens app-side.
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
    crate::ipc::exchange_at(&path, req)
}

/// The unreachable/no-crew message + exit code, shared by both subcommands.
fn no_crew() -> (String, i32) {
    ("NO_ANSWER: unreachable (no crew running)".to_string(), 3)
}

/// `crew ask <to> "<question>"`. `<to>` may be `pane@instance` to reach an
/// agent in another crew instance's pane (v3 federation) — the instance part
/// picks the socket, the pane part is resolved by that crew unchanged.
pub(crate) fn run_ask(to: &str, question: &str) -> i32 {
    let (pane, target) = crate::askaddr::resolve_target(to);
    let from = std::env::var("CREW_PANE").unwrap_or_else(|_| "an agent".to_string());
    let id = format!("q{}", std::process::id());
    let req = Request::Ask {
        v: PROTOCOL_V,
        from,
        to: pane.to_string(),
        question: question.to_string(),
        id,
    };
    let (text, code) = match target {
        crate::askaddr::Target::Local(inst) => exchange(&req, inst.as_deref())
            .map(|r| render(&r))
            .unwrap_or_else(no_crew),
        crate::askaddr::Target::Remote {
            host,
            port,
            instance,
        } => dial_remote(&host, port, &instance, &req),
    };
    println!("{text}");
    code
}

/// Reach a remote crew's pane via the relay, using the shared
/// `CREW_FEDERATE_TOKEN` (unset → federation is off on this side).
fn dial_remote(host: &str, port: u16, instance: &str, req: &Request) -> (String, i32) {
    let Some(token) = std::env::var("CREW_FEDERATE_TOKEN")
        .ok()
        .filter(|t| !t.is_empty())
    else {
        return (
            "NO_ANSWER: set CREW_FEDERATE_TOKEN to reach a remote crew".to_string(),
            3,
        );
    };
    crate::relay::dial(host, port, Some(instance), req, &token)
        .map(|r| render(&r))
        .unwrap_or_else(|| {
            (
                format!("NO_ANSWER: unreachable (no relay at crew://{host}:{port})"),
                3,
            )
        })
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

/// `crew federate` — show whether cross-host federation is on and how to use it.
pub(crate) fn run_federate() -> i32 {
    let token = std::env::var("CREW_FEDERATE_TOKEN").ok();
    let bind = std::env::var("CREW_FEDERATE_BIND").unwrap_or_else(|_| "0.0.0.0".into());
    let port = std::env::var("CREW_FEDERATE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(crate::askaddr::DEFAULT_RELAY_PORT);
    println!("{}", federate_status(token.as_deref(), &bind, port));
    0
}

/// The `crew federate` status text — pure, so it's testable.
fn federate_status(token: Option<&str>, bind: &str, port: u16) -> String {
    if token.is_some_and(|t| !t.is_empty()) {
        format!(
            "federation: ON\n  listening {bind}:{port} (token required)\n  \
             reach me: crew ask <pane>@crew://<this-host>/<instance> \"…\"  \
             (callers need the same CREW_FEDERATE_TOKEN)\n  \
             note: token + data are plaintext — run over a trusted net or a tunnel/TLS"
        )
    } else {
        format!(
            "federation: OFF (nothing is exposed)\n  enable: \
             CREW_FEDERATE_TOKEN=<secret> crew   (binds a relay on {bind}:{port})\n  \
             callers then: crew ask <pane>@crew://<host>/<instance> \"…\" with the same token"
        )
    }
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
        Some("federate") => Some(run_federate()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn federate_status_reflects_the_token() {
        let on = federate_status(Some("s3cret"), "0.0.0.0", 7733);
        assert!(on.contains("ON") && on.contains("0.0.0.0:7733") && on.contains("crew://"));
        let off = federate_status(None, "0.0.0.0", 7733);
        assert!(off.contains("OFF") && off.contains("CREW_FEDERATE_TOKEN"));
        assert!(federate_status(Some(""), "0.0.0.0", 7733).contains("OFF"));
    }
}
