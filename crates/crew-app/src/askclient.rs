//! Client side of inter-pane `ask`: the `crew ask <to> "<q>"` and `crew panes`
//! subcommands. They connect to the running GUI's IPC socket, send one
//! request, print the reply, and exit — short-circuited in `main.rs` before
//! any GUI init (like `--list-fonts`). All the waiting happens app-side.
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

use crate::ipc_types::{NoAnswer, PaneCard, Reply, Request, PROTOCOL_V};

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
    }
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

/// Send one request over the socket and read the reply. `None` if no crew is
/// listening (socket absent / refused).
fn exchange(req: &Request) -> Option<Reply> {
    let mut stream = UnixStream::connect(crate::ipc::socket_path()).ok()?;
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

/// `crew ask <to> "<question>"`.
pub(crate) fn run_ask(to: &str, question: &str) -> i32 {
    let from = std::env::var("CREW_PANE").unwrap_or_else(|_| "an agent".to_string());
    let id = format!("q{}", std::process::id());
    let req = Request::Ask {
        v: PROTOCOL_V,
        from,
        to: to.to_string(),
        question: question.to_string(),
        id,
    };
    let (text, code) = exchange(&req).map(|r| render(&r)).unwrap_or_else(no_crew);
    println!("{text}");
    code
}

/// `crew panes`.
pub(crate) fn run_panes() -> i32 {
    let (text, code) = exchange(&Request::Panes { v: PROTOCOL_V })
        .map(|r| render(&r))
        .unwrap_or_else(no_crew);
    println!("{text}");
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_verdicts_and_codes() {
        let (t, c) = render(&Reply::Answered { text: "v2".into() });
        assert_eq!((t.as_str(), c), ("ANSWERED: v2", 0));

        let (t, c) = render(&Reply::NoAnswer {
            reason: NoAnswer::IdleNoEngage,
            partial: None,
        });
        assert!(t.contains("NO_ANSWER") && t.contains("idle"));
        assert_eq!(c, 2);

        let (t, c) = render(&Reply::NoAnswer {
            reason: NoAnswer::Unreachable,
            partial: None,
        });
        assert_eq!(c, 3, "unreachable is code 3: {t}");

        let (t, _) = render(&Reply::NoAnswer {
            reason: NoAnswer::Stalled,
            partial: Some("half an answer".into()),
        });
        assert!(t.contains("partial") && t.contains("half an answer"));
    }

    #[test]
    fn render_roster_is_a_table_with_ids() {
        let out = render_roster(&[PaneCard {
            id: "p2".into(),
            label: Some("schema".into()),
            kind: "terminal".into(),
            running: Some("claude".into()),
            dir: Some("db".into()),
            busy: false,
        }]);
        assert!(out.contains("p2") && out.contains("schema") && out.contains("claude"));
        assert!(out.contains("idle"));
    }
}
