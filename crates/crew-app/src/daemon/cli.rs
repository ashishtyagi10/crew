//! `crew daemon …` — the CLI face of the resident. Dispatched in `main` beside the ask client,
//! BEFORE any GUI initialization: `crew daemon run` must never open a window, and `crew daemon
//! status` must be answerable on a headless box.
use super::probe;
use crate::ipc_types::{Reply, Request, PROTOCOL_V};

/// Usage text for a malformed `crew daemon` invocation.
const USAGE: &str = "\
usage:
  crew daemon run             run the resident in this process (foreground)
  crew daemon status          report a running resident, or exit 3 if there is none
  crew daemon open [label]    open an agent session the daemon owns (--cwd DIR)
  crew daemon sessions        list the daemon's sessions
  crew daemon close <id>      stop one session
";

/// The instance this process addresses, matching the ask client's rule so a user who sets
/// `CREW_INSTANCE` gets one coherent world (their GUI, their ask socket, their daemon).
fn instance() -> Option<String> {
    std::env::var("CREW_INSTANCE")
        .ok()
        .filter(|s| !s.is_empty())
}

/// The "nothing is listening" message and exit code, shared by every subcommand that needs a
/// running resident.
fn no_daemon() -> i32 {
    println!("no crew daemon running");
    3
}

/// Route `crew daemon <sub>`. `None` when this invocation is not a daemon subcommand, so `main`
/// falls through to the GUI launch exactly as before.
pub(crate) fn dispatch_cli() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("daemon") {
        return None;
    }
    Some(run_sub(&args[1..]))
}

/// The value of `--cwd`, if given.
fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let i = args.iter().position(|a| a == name)?;
    args.get(i + 1).map(String::as_str)
}

/// The first argument that is not a flag or a flag's value — the optional label.
fn positional(args: &[String], after: usize) -> Option<&str> {
    let mut it = args.iter().skip(after);
    while let Some(a) = it.next() {
        if a.starts_with("--") {
            it.next();
            continue;
        }
        return Some(a);
    }
    None
}

/// Render a reply that is not the one this subcommand expected, so a protocol mismatch reads as
/// a mismatch instead of a silent zero exit.
fn unexpected(r: &Reply) -> i32 {
    println!("unexpected reply from the daemon: {r:?}");
    4
}

/// Testable core: the daemon subcommand and its arguments in, the process exit code out.
pub(crate) fn run_sub(args: &[String]) -> i32 {
    let inst = instance();
    let sub = args.first().map(String::as_str);
    match sub {
        Some("run") => super::run(inst.as_deref()),
        Some("open") => {
            let label = positional(args, 1).unwrap_or("crew");
            let req = Request::OpenSession {
                v: PROTOCOL_V,
                label: label.to_string(),
                cwd: flag(args, "--cwd").map(str::to_string),
            };
            match super::request(inst.as_deref(), &req) {
                Some(Reply::Session { id }) => {
                    println!("{id}");
                    0
                }
                Some(Reply::Failed { message }) => {
                    println!("{message}");
                    1
                }
                Some(other) => unexpected(&other),
                None => no_daemon(),
            }
        }
        Some("sessions") => {
            match super::request(inst.as_deref(), &Request::Sessions { v: PROTOCOL_V }) {
                Some(Reply::Sessions { sessions }) => {
                    if sessions.is_empty() {
                        println!("no sessions");
                    }
                    for s in sessions {
                        let state = if s.alive { "running" } else { "dead" };
                        let cwd = s.cwd.unwrap_or_default();
                        println!("{}  {}  {}  {}", s.id, state, s.label, cwd);
                    }
                    0
                }
                Some(other) => unexpected(&other),
                None => no_daemon(),
            }
        }
        Some("close") => {
            let Some(id) = positional(args, 1) else {
                print!("{USAGE}");
                return 2;
            };
            let req = Request::CloseSession {
                v: PROTOCOL_V,
                id: id.to_string(),
            };
            match super::request(inst.as_deref(), &req) {
                Some(Reply::Closed { id, was_alive }) => {
                    println!(
                        "{id} closed{}",
                        if was_alive {
                            ""
                        } else {
                            " (it had already died)"
                        }
                    );
                    0
                }
                Some(Reply::Failed { message }) => {
                    println!("{message}");
                    1
                }
                Some(other) => unexpected(&other),
                None => no_daemon(),
            }
        }
        Some("status") => match probe(inst.as_deref()) {
            Some(st) => {
                println!(
                    "crew daemon {} running — pid {}, up {}s, {} session(s)",
                    st.version, st.pid, st.uptime_s, st.sessions
                );
                0
            }
            None => no_daemon(),
        },
        _ => {
            print!("{USAGE}");
            2
        }
    }
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
