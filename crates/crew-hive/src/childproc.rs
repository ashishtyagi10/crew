//! One rule for every child process crew starts: **no console window**.
//!
//! ## The bug this exists to prevent
//!
//! `crew.exe` is a *console-subsystem* binary (`Subsystem = WINDOWS_CUI` in the
//! PE header) — it has to be, because `crew --version`, `crew ask …`,
//! `--list-fonts` and the `--broker-plugin` stdio loop all need real stdio.
//!
//! The GUI process, though, is launched with `DETACHED_PROCESS` (see the app's
//! `detach` module) so closing the launching shell cannot take the window with
//! it. A detached process has **no console at all**. And on Windows, when a
//! process with no console starts a console application, the OS *allocates a
//! brand-new console window for the child*.
//!
//! crew polls `git status` every three seconds for the sidebar. On Windows that
//! meant **a new terminal window every three seconds, forever**, stealing focus
//! from the app each time — while the main window sat there looking fine. The
//! broker, the shell probe, and every MCP server did the same thing on startup.
//!
//! `CREATE_NO_WINDOW` is the fix: the child still gets a console (so its stdio
//! pipes work exactly as before), it just gets an *invisible* one.
//!
//! ## Why it lives here
//!
//! Both `crew-app` (sidebar git, shell probe) and `crew-plugin` (broker spawn,
//! MCP servers, the broker's own git calls) start children, and `crew-hive` is
//! the crate they both already depend on. A single helper is the point: the
//! failure mode is *forgetting* one call site, so there is also a guard test
//! (`no_console_window_is_applied_at_every_spawn_site`) that reads the source
//! tree and fails on a `Command::new` that does not route through here.
//!
//! Panes are not affected and must not use this: they run under portable-pty's
//! ConPTY, which gives the child a pseudoconsole rather than a window.
use std::process::Command;

/// Windows `CREATE_NO_WINDOW`: give the child an invisible console instead of
/// letting the OS pop a new terminal window for it.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Start `cmd` without a console window. No-op off Windows.
///
/// Call this on **every** `Command` crew spawns outside a pty. Returns `cmd`
/// so it chains: `no_console_window(Command::new("git")).arg("-C")…`.
#[cfg(windows)]
pub fn no_console_window(cmd: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(CREATE_NO_WINDOW)
}

/// Unix has no such concept — a child inherits the terminal or nothing.
#[cfg(not(windows))]
pub fn no_console_window(cmd: &mut Command) -> &mut Command {
    cmd
}

#[cfg(test)]
#[path = "childproc_tests.rs"]
mod tests;
