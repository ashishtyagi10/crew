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
    /// Drag reporting (DECSET 1002): a move with a button held is reported.
    pub mouse_drag: bool,
    /// Any-motion reporting (DECSET 1003): every move is reported, button or
    /// not, so a program can light what the pointer hovers (btop, k9s).
    pub mouse_motion: bool,
    /// Mouse events should be SGR-encoded (`ESC [ < … M`) rather than legacy X10.
    pub sgr_mouse: bool,
    /// SGR-Pixels (DECSET 1016): with a mouse mode on, reports carry pixel
    /// coordinates in the 1006 form. Tracked by crew's own scanner
    /// (`pixelmouse`); it takes precedence over `sgr_mouse` and X10.
    pub pixel_mouse: bool,
    /// Application cursor keys (DECCKM): arrows are `ESC O A` not `ESC [ A`.
    pub app_cursor: bool,
    /// Alternate-scroll (DECSET 1007): translate the wheel to arrow keys in the
    /// alternate screen. Enabled by default in xterm-compatible terminals.
    pub alternate_scroll: bool,
    /// Focus reporting (DECSET 1004): the program wants `CSI I` / `CSI O`
    /// when its pane gains or loses keyboard focus (see `focusreport`).
    pub focus_events: bool,
    /// Any kitty keyboard protocol flag is on (`kitty_flags != 0`): the
    /// program wants keys as `CSI … u`, and a killed one left that behind
    /// (`promptmodes`).
    pub kitty_keys: bool,
    /// The active kitty keyboard flags, alacritty's stack top: 1 disambiguate
    /// escape codes, 2 report event types (press/repeat/release), 4 report
    /// alternate keys, 8 report all keys as escape codes, 16 report
    /// associated text. The app's `keykitty` / `kittyreport` encode by them.
    pub kitty_flags: u8,
}
