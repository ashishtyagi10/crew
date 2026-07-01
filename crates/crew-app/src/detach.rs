//! `--detach` / `-d`: relaunch crew in a new session, detached from the
//! controlling terminal, so closing the launching shell doesn't SIGHUP it.
//!
//! We re-exec a fresh copy of the binary (rather than `fork`) because the GUI
//! toolkit (winit / AppKit) must not be initialised across a `fork`. The child
//! carries `CREW_DETACHED=1` so it runs the GUI instead of detaching again.
use std::process::{Command, Stdio};

/// Env marker set on the detached child so it doesn't detach a second time.
const DETACHED_ENV: &str = "CREW_DETACHED";

/// Whether `--detach` / `-d` appears in `args`.
fn has_detach_flag<I: IntoIterator<Item = String>>(args: I) -> bool {
    args.into_iter().any(|a| a == "--detach" || a == "-d")
}

/// `args` with the detach flags removed — the child is launched with the rest.
fn strip_detach_flags<I: IntoIterator<Item = String>>(args: I) -> Vec<String> {
    args.into_iter()
        .filter(|a| a != "--detach" && a != "-d")
        .collect()
}

/// True when this process is the already-detached child (don't detach again).
pub fn is_detached_child() -> bool {
    std::env::var_os(DETACHED_ENV).is_some()
}

/// Whether `--detach` / `-d` was requested on the command line.
pub fn wants_detach() -> bool {
    has_detach_flag(std::env::args().skip(1))
}

/// Spawn a detached copy of ourselves (new session, stdio → null) and return;
/// `main` then exits the parent, freeing the terminal while the GUI runs on.
pub fn relaunch_detached() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    let args = strip_detach_flags(std::env::args().skip(1));
    let mut cmd = Command::new(exe);
    cmd.args(&args)
        .env(DETACHED_ENV, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    detach_session(&mut cmd);
    let child = cmd.spawn()?;
    println!(
        "crew detached (pid {}) — safe to close this terminal",
        child.id()
    );
    Ok(())
}

#[cfg(unix)]
fn detach_session(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    // New session (setsid) → no controlling terminal → the child is not in the
    // launching shell's session, so the terminal's SIGHUP on close can't reach it.
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn detach_session(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    // Detach from the parent console and start a new process group so the
    // console window can close without taking the GUI process with it.
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_both_flag_spellings() {
        assert!(has_detach_flag(["--detach".to_string()]));
        assert!(has_detach_flag(["-d".to_string()]));
        assert!(has_detach_flag(["run".to_string(), "-d".to_string()]));
        assert!(!has_detach_flag(["--depth".to_string(), "x".to_string()]));
        assert!(!has_detach_flag(Vec::<String>::new()));
    }

    #[test]
    fn strips_only_the_detach_flags() {
        let args = [
            "-d".to_string(),
            "--self-update".to_string(),
            "x".to_string(),
        ];
        assert_eq!(strip_detach_flags(args), vec!["--self-update", "x"]);
        let clean = ["--broker-plugin".to_string()];
        assert_eq!(strip_detach_flags(clean.clone()), vec!["--broker-plugin"]);
    }
}
