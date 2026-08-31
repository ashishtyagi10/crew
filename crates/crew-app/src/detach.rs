//! Detached launch (the **default**): relaunch crew in a new session, detached
//! from the controlling terminal, so closing the launching shell doesn't
//! SIGHUP it. `--no-detach` / `--foreground` keeps crew attached (handy for
//! debugging with visible logs); `--detach` / `-d` is still accepted as a
//! no-op for existing scripts.
//!
//! We re-exec a fresh copy of the binary (rather than `fork`) because the GUI
//! toolkit (winit / AppKit) must not be initialised across a `fork`. The child
//! carries `CREW_DETACHED=1` so it runs the GUI instead of detaching again.
use std::process::{Command, Stdio};

/// Env marker set on the detached child so it doesn't detach a second time.
const DETACHED_ENV: &str = "CREW_DETACHED";

/// Flags this module owns; all are stripped from the relaunched child's args.
const DETACH_FLAGS: [&str; 4] = ["--detach", "-d", "--no-detach", "--foreground"];

/// Whether `--no-detach` / `--foreground` appears in `args` (stay attached).
fn has_foreground_flag<I: IntoIterator<Item = String>>(args: I) -> bool {
    args.into_iter()
        .any(|a| a == "--no-detach" || a == "--foreground")
}

/// `args` with the detach flags removed — the child is launched with the rest.
fn strip_detach_flags<I: IntoIterator<Item = String>>(args: I) -> Vec<String> {
    args.into_iter()
        .filter(|a| !DETACH_FLAGS.contains(&a.as_str()))
        .collect()
}

/// True when this process is the already-detached child (don't detach again).
pub fn is_detached_child() -> bool {
    std::env::var_os(DETACHED_ENV).is_some()
}

/// Detaching is the default; `--no-detach` / `--foreground` opts out.
pub fn should_detach() -> bool {
    !has_foreground_flag(std::env::args().skip(1))
}

/// The (exe, args) a restart will spawn: OUR OWN PATH with detach flags
/// stripped. self_update replaces the file at this path atomically, so
/// re-exec here is the guarantee that `/update`'s relaunch (and any other)
/// always loads the newest installed binary.
pub fn restart_command() -> anyhow::Result<(std::path::PathBuf, Vec<String>)> {
    let exe = std::env::current_exe()?;
    let args = strip_detach_flags(std::env::args().skip(1));
    Ok((exe, args))
}

/// Where the detached child's stderr goes. A file if we can open one, else
/// null as before.
///
/// This used to be unconditionally `/dev/null`, which meant a detached crew
/// threw away every panic message, every wgpu/winit warning, and the
/// `eprintln!`s this crate already writes on GPU-init and socket failures. A
/// crash then left NOTHING behind — see [`crate::crashlog`]. Appending (never
/// truncating) matters: the log has to survive the restart that follows a
/// crash, or it is erased by exactly the event it exists to explain.
fn child_stderr() -> Stdio {
    let Some(path) = crate::crashlog::stderr_path() else {
        return Stdio::null();
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_or_else(|_| Stdio::null(), Stdio::from)
}

/// Spawn a detached copy of ourselves (new session, stderr → log) and return
/// its pid — shared by the detached launch path and `/update`'s relaunch.
pub fn spawn_detached_copy() -> anyhow::Result<u32> {
    let (exe, args) = restart_command()?;
    let mut cmd = Command::new(exe);
    cmd.args(&args)
        .env(DETACHED_ENV, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(child_stderr());
    detach_session(&mut cmd);
    Ok(cmd.spawn()?.id())
}

/// Spawn a detached copy of ourselves and return; `main` then exits the
/// parent, freeing the terminal while the GUI runs on.
pub fn relaunch_detached() -> anyhow::Result<()> {
    let pid = spawn_detached_copy()?;
    println!("crew detached (pid {pid}) — safe to close this terminal");
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
#[path = "detach_tests.rs"]
mod tests;
