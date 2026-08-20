//! Windows only: give the CLI back the console that the GUI subsystem takes
//! away.
//!
//! ## Why crew is a GUI-subsystem binary
//!
//! A *console*-subsystem program gets a console window from Windows before its
//! first instruction runs. Launching crew from the Start menu or Explorer
//! therefore flashed a black window every time — and nothing the program does
//! can prevent that, only end it early. `FreeConsole` on the first line still
//! leaves the window up for however long process init takes, which for a 24 MB
//! binary is plainly visible.
//!
//! So `main.rs` declares `#![windows_subsystem = "windows"]` and no console is
//! ever created.
//!
//! ## What that costs, and what this module buys back
//!
//! A GUI-subsystem process starts with **no standard handles** when launched
//! from a terminal, so `crew --version`, `crew ask …`, `crew panes` and
//! `--list-fonts` would print into nothing. [`attach_to_parent`] fixes that:
//! it attaches to the console the launching shell already owns and points the
//! process's std handles at it.
//!
//! Two rules it must follow, both of which are easy to get wrong:
//!
//! 1. **Never clobber a handle that is already good.** When crew is run with
//!    its output piped or redirected — `crew --list-fonts > fonts.txt`, and
//!    every broker spawn, which talks a JSON-line protocol over pipes — the
//!    shell has already installed the right handles. Overwriting them with
//!    `CONOUT$` would send the output to the console instead of the pipe and
//!    silently break both. Hence [`needs_console_handle`].
//! 2. **The GUI process must not attach.** It is launched `DETACHED_PROCESS`
//!    precisely so a closed terminal cannot reach it; attaching it back to
//!    that terminal's console would undo that.
//!
//! ## The one behaviour that genuinely changes
//!
//! Shells do not *wait* for GUI-subsystem programs. `crew --version` typed at
//! a prompt still prints, but the prompt may return first and the output land
//! under it. That is inherent to the subsystem choice — there is no flag that
//! makes a shell wait for a GUI binary — and it is the price of never flashing
//! a console on a normal launch.

/// Whether `handle` (a raw `HANDLE` as `isize`) needs to be pointed at the
/// console, i.e. the process does not already have a usable one.
///
/// `GetStdHandle` reports "no handle" as either null or `INVALID_HANDLE_VALUE`
/// (`-1`), and a GUI-subsystem process launched from Explorer gets null. Any
/// other value is a real handle — a pipe, a file, or an inherited console —
/// and must be left exactly as it is.
///
/// Pure and platform-independent so the rule itself is testable off Windows;
/// it is the rule, not the FFI, that breaks piped output when it is wrong.
// Only `imp` (Windows) and the tests call this; on other platforms it is the
// documented rule with no caller, which is still worth keeping and testing.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn needs_console_handle(handle: isize) -> bool {
    handle == 0 || handle == -1
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::{
        AttachConsole, GetStdHandle, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE,
        STD_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };

    /// Point one std handle at the console device, but only if the process has
    /// no usable handle there already — see the module docs on piped output.
    fn adopt(which: STD_HANDLE, device: &str, write: bool) {
        unsafe {
            if !super::needs_console_handle(GetStdHandle(which) as isize) {
                return;
            }
            let wide: Vec<u16> = std::ffi::OsStr::new(device)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let handle = CreateFileW(
                wide.as_ptr(),
                if write { GENERIC_WRITE } else { GENERIC_READ },
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            );
            if !handle.is_null() && handle != INVALID_HANDLE_VALUE {
                SetStdHandle(which, handle);
            }
        }
    }

    /// Attach to the launching shell's console, if there is one, and wire up
    /// whichever std handles are still missing. Silent no-op otherwise — a
    /// launch from Explorer has no parent console and wants none.
    pub(crate) fn attach_to_parent() {
        // Fails when the parent has no console (Explorer, the Start menu, a
        // service). That is the ordinary GUI launch, and it is not an error.
        if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } == 0 {
            return;
        }
        adopt(STD_OUTPUT_HANDLE, "CONOUT$", true);
        adopt(STD_ERROR_HANDLE, "CONOUT$", true);
        adopt(STD_INPUT_HANDLE, "CONIN$", false);
    }
}

/// Attach this process to the console of whatever launched it, so CLI output
/// is visible. No-op off Windows, where stdio needs no help.
pub(crate) fn attach_to_parent() {
    #[cfg(windows)]
    imp::attach_to_parent();
}

#[cfg(test)]
#[path = "wincon_tests.rs"]
mod tests;
