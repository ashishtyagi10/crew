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
  crew daemon send <id> <ln>  write one line to a session's agent
  crew daemon poll <id>       read a session's output (--after N to resume)
  crew daemon channels        list the ways in, and which are usable
  crew daemon say <to> <txt>  send a message out through a channel (kind:rest)
  crew daemon install         start the resident at login (opt-in; --remove undoes it)
";

/// The instance this process addresses, matching the ask client's rule so a user who sets
/// `CREW_INSTANCE` gets one coherent world (their GUI, their ask socket, their daemon).
fn instance() -> Option<String> {
    std::env::var("CREW_INSTANCE")
        .ok()
        .filter(|s| !s.is_empty())
}

/// Whether the opt-in login service is installed for this user.
fn service_state() -> &'static str {
    let Some(home) = dirs::home_dir() else {
        return "unknown (no home directory)";
    };
    match std::env::current_exe()
        .ok()
        .and_then(|exe| super::service::unit_for_host(&exe))
    {
        Some(unit) if super::service::is_installed(&home, &unit) => "installed",
        Some(_) => "not installed (crew daemon install)",
        None => "unsupported on this platform",
    }
}

/// `crew daemon install` / `--remove`. Nothing else in crew may call this: a background service
/// the user did not ask for turns a bad release into a login loop instead of an `/update`.
fn install(remove: bool) -> i32 {
    let Some(home) = dirs::home_dir() else {
        println!("cannot find your home directory");
        return 1;
    };
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            println!("cannot locate the crew binary: {e}");
            return 1;
        }
    };
    let Some(unit) = super::service::unit_for_host(&exe) else {
        println!("crew has no service integration for this platform yet");
        return 1;
    };
    if remove {
        if let Err(e) = super::service::run_step(&home, &unit.deactivate) {
            println!("could not deactivate the service: {e}");
        }
        return match super::service::remove_unit(&home, &unit) {
            Ok(true) => {
                println!("the crew daemon will no longer start at login");
                0
            }
            Ok(false) => {
                println!("the crew daemon was not installed");
                0
            }
            Err(e) => {
                println!("could not remove the service file: {e}");
                1
            }
        };
    }
    match super::service::write_unit(&home, &unit) {
        Ok(path) => {
            println!("wrote {}", path.display());
            match super::service::run_step(&home, &unit.activate) {
                Ok(()) => println!("the crew daemon will start at login (and is starting now)"),
                Err(e) => println!(
                    "wrote the service file but could not activate it: {e}\n\
                     activate it yourself with: {}",
                    unit.activate.join(" ")
                ),
            }
            0
        }
        Err(e) => {
            println!("could not write the service file: {e}");
            1
        }
    }
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
        Some("send") => {
            let (Some(id), Some(line)) = (positional(args, 1), positional(args, 2)) else {
                print!("{USAGE}");
                return 2;
            };
            let req = Request::SessionSend {
                v: PROTOCOL_V,
                id: id.to_string(),
                line: line.to_string(),
            };
            match super::request(inst.as_deref(), &req) {
                Some(Reply::Sent {
                    delivered: true, ..
                }) => 0,
                Some(Reply::Sent {
                    id,
                    delivered: false,
                }) => {
                    println!("{id}: not delivered (the process is gone)");
                    1
                }
                Some(Reply::Failed { message }) => {
                    println!("{message}");
                    1
                }
                Some(other) => unexpected(&other),
                None => no_daemon(),
            }
        }
        Some("poll") => {
            let Some(id) = positional(args, 1) else {
                print!("{USAGE}");
                return 2;
            };
            let after = flag(args, "--after")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let req = Request::SessionPoll {
                v: PROTOCOL_V,
                id: id.to_string(),
                after,
            };
            match super::request(inst.as_deref(), &req) {
                Some(Reply::Events {
                    lines,
                    next,
                    dropped,
                }) => {
                    if dropped > after {
                        println!(
                            "[{} earlier line(s) dropped from the buffer]",
                            dropped - after
                        );
                    }
                    for l in lines {
                        println!("{l}");
                    }
                    eprintln!("next cursor: {next}");
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
        Some("channels") => {
            match super::request(inst.as_deref(), &Request::Channels { v: PROTOCOL_V }) {
                Some(Reply::Channels { registered, ready }) => {
                    if registered.is_empty() {
                        println!("no channels — crew is reachable from a pane only");
                    }
                    for k in registered {
                        let state = if ready.contains(&k) {
                            "ready"
                        } else {
                            "not configured"
                        };
                        println!("{k}  {state}");
                    }
                    0
                }
                Some(other) => unexpected(&other),
                None => no_daemon(),
            }
        }
        Some("say") => {
            let (Some(to), Some(text)) = (positional(args, 1), positional(args, 2)) else {
                print!("{USAGE}");
                return 2;
            };
            let req = Request::Say {
                v: PROTOCOL_V,
                to: to.to_string(),
                text: text.to_string(),
            };
            match super::request(inst.as_deref(), &req) {
                Some(Reply::Sent {
                    delivered: true, ..
                }) => 0,
                Some(Reply::Failed { message }) => {
                    println!("{message}");
                    1
                }
                Some(other) => unexpected(&other),
                None => no_daemon(),
            }
        }
        Some("install") => install(args.iter().any(|a| a == "--remove")),
        Some("uninstall") => install(true),
        Some("status") => {
            let code = match probe(inst.as_deref()) {
                Some(st) => {
                    println!(
                        "crew daemon {} running — pid {}, up {}s, {} session(s)",
                        st.version, st.pid, st.uptime_s, st.sessions
                    );
                    0
                }
                None => no_daemon(),
            };
            // "Running now" and "comes back at login" are different questions, and the second
            // is the one that decides whether this is a resident or a process you started once.
            println!("login service: {}", service_state());
            code
        }
        _ => {
            print!("{USAGE}");
            2
        }
    }
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
