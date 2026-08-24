//! `crew daemon …` — the CLI face of the resident. Dispatched in `main` beside the ask client,
//! BEFORE any GUI initialization: `crew daemon run` must never open a window, and `crew daemon
//! status` must be answerable on a headless box.
use super::probe;

/// Usage text for a malformed `crew daemon` invocation.
const USAGE: &str = "\
usage:
  crew daemon run      run the resident in this process (foreground)
  crew daemon status   report a running resident, or exit 3 if there is none
";

/// The instance this process addresses, matching the ask client's rule so a user who sets
/// `CREW_INSTANCE` gets one coherent world (their GUI, their ask socket, their daemon).
fn instance() -> Option<String> {
    std::env::var("CREW_INSTANCE")
        .ok()
        .filter(|s| !s.is_empty())
}

/// Route `crew daemon <sub>`. `None` when this invocation is not a daemon subcommand, so `main`
/// falls through to the GUI launch exactly as before.
pub(crate) fn dispatch_cli() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("daemon") {
        return None;
    }
    Some(run_sub(args.get(1).map(String::as_str)))
}

/// Testable core: the subcommand word in, the process exit code out.
pub(crate) fn run_sub(sub: Option<&str>) -> i32 {
    let inst = instance();
    match sub {
        Some("run") => super::run(inst.as_deref()),
        Some("status") => match probe(inst.as_deref()) {
            Some(st) => {
                println!(
                    "crew daemon {} running — pid {}, up {}s, {} session(s)",
                    st.version, st.pid, st.uptime_s, st.sessions
                );
                0
            }
            None => {
                println!("no crew daemon running");
                3
            }
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
