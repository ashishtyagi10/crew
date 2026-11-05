//! A snapshot of the DEC private modes a terminal program has enabled that
//! affect how Crew should route a scroll wheel. Full-screen apps (an alternate
//! screen) have no scrollback of their own, so the wheel must be forwarded to
//! the program — as mouse-wheel events when it requested mouse reporting, or as
//! arrow keys under xterm "alternate scroll" otherwise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputModes {
    /// The alternate screen buffer is active (a full-screen TUI like vim/claude).
    pub alt_screen: bool,
    /// The program requested mouse reporting (any of click/drag/motion).
    pub mouse: bool,
    /// Mouse events should be SGR-encoded (`ESC [ < … M`) rather than legacy X10.
    pub sgr_mouse: bool,
    /// Application cursor keys (DECCKM): arrows are `ESC O A` not `ESC [ A`.
    pub app_cursor: bool,
    /// Alternate-scroll (DECSET 1007): translate the wheel to arrow keys in the
    /// alternate screen. Enabled by default in xterm-compatible terminals.
    pub alternate_scroll: bool,
    /// Focus reporting (DECSET 1004): the program wants `CSI I` / `CSI O`
    /// when its pane gains or loses keyboard focus (see `focusreport`).
    pub focus_events: bool,
}
