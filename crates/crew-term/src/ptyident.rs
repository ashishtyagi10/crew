//! Who a pane's programs are talking to: crew, saying so.
//!
//! A shell in a pane inherits crew's own environment, and crew's is whatever
//! started it. Launched from the Dock it has no `COLORTERM`, so programs that
//! look for one (chalk, and Claude Code with it) drew in 256 colours on a
//! terminal that draws every 24-bit colour. Launched from another terminal it
//! carried THAT terminal's name — `TERM_PROGRAM=iTerm.app`, `KITTY_WINDOW_ID`,
//! `TMUX` — so programs reached for its escape codes, or believed they were
//! inside a tmux that was not there. Every pane now names crew and drops the
//! names other terminals left behind, as each of those terminals does for
//! itself.
use portable_pty::CommandBuilder;

/// The variables a terminal (or multiplexer) sets to name itself to the
/// programs it runs — none of them true of a crew pane.
const FOREIGN: &[&str] = &[
    "TERM_PROGRAM",
    "TERM_PROGRAM_VERSION",
    "TERM_SESSION_ID",
    "LC_TERMINAL",
    "LC_TERMINAL_VERSION",
    "ITERM_SESSION_ID",
    "ITERM_PROFILE",
    "KITTY_WINDOW_ID",
    "KITTY_PID",
    "KITTY_PUBLIC_KEY",
    "KITTY_LISTEN_ON",
    "WEZTERM_PANE",
    "WEZTERM_UNIX_SOCKET",
    "WEZTERM_EXECUTABLE",
    "GHOSTTY_RESOURCES_DIR",
    "GHOSTTY_BIN_DIR",
    "GHOSTTY_SHELL_FEATURES",
    "ALACRITTY_SOCKET",
    "ALACRITTY_WINDOW_ID",
    "WT_SESSION",
    "WT_PROFILE_ID",
    "VTE_VERSION",
    "KONSOLE_VERSION",
    "TERMINAL_EMULATOR",
    "WARP_IS_LOCAL_SHELL_SESSION",
    "TMUX",
    "TMUX_PANE",
    "STY",
];

/// `cmd` told it runs in crew: 24-bit colour, crew's name and version, and
/// no other terminal's.
pub(crate) fn identify(cmd: &mut CommandBuilder) {
    for k in FOREIGN {
        cmd.env_remove(k);
    }
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "crew");
    cmd.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
}

#[cfg(test)]
#[path = "ptyident_tests.rs"]
mod tests;
