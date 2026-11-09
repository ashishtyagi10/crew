//! Viewport, selection, title/cwd/clipboard accessors plus the `TermModel`
//! and `Drop` impls — child module of `pty`, split for the 200-line cap.
use super::*;

impl PtyTerm {
    /// Scroll the viewport by `delta` lines into scrollback (positive = older).
    pub fn scroll(&mut self, delta: i32) {
        self.core.scroll(delta);
    }

    /// Jump back to the live bottom of the terminal.
    pub fn scroll_to_bottom(&mut self) {
        self.core.scroll_to_bottom();
    }

    /// The live cursor's viewport cell — `None` when the program hid it or
    /// the view is scrolled into history. What a caret animation follows.
    pub fn cursor_cell(&self) -> Option<(u16, u16)> {
        self.core.cursor_cell()
    }

    /// Publish the frame's cell size in pixels (see [`PlacedImage::cells`]),
    /// telling the pty when it changes the text area's pixel size.
    pub fn set_cell_px(&mut self, w: u32, h: u32) {
        let before = self.core.pty_size();
        self.core.set_cell_px(w, h);
        if self.core.pty_size() != before {
            let _ = self.master.resize(self.core.pty_size());
        }
    }

    /// Lines of scrollback above the screen.
    pub fn history_lines(&self) -> usize {
        self.core.history_lines()
    }

    /// The absolute line of the live screen's top row — what maps a
    /// picture's anchor (see [`crate::PlacedImage::line`]) onto the row it
    /// is drawn at now. Unlike `history_lines`, it keeps counting once the
    /// scrollback is full.
    pub fn screen_line(&self) -> u64 {
        self.core.screen_line()
    }

    /// Every line this pane's buffer has ever held — `scrollable_lines` plus
    /// what the scrollback has thrown away. Command marks and the unread
    /// count are kept in these numbers: `scrollable_lines` stops growing once
    /// the scrollback is full, which froze both.
    pub fn lines_seen(&self) -> usize {
        self.core.lines_seen() as usize
    }

    /// Lines currently scrolled back from the live bottom (0 = at the bottom).
    pub fn display_offset(&self) -> usize {
        self.core.display_offset()
    }

    /// Every line the viewport can reach — scrollback plus the live screen.
    pub fn scrollable_lines(&self) -> usize {
        self.core.scrollable_lines()
    }

    /// Whether the program enabled bracketed-paste mode.
    pub fn bracketed_paste(&self) -> bool {
        self.core.bracketed_paste()
    }

    /// The DEC private modes that decide how a scroll wheel is routed (alternate
    /// screen, mouse reporting, app-cursor keys).
    pub fn input_modes(&self) -> crate::modes::InputModes {
        self.core.input_modes()
    }

    /// Begin a mouse selection at viewport cell (col, row); `block` = rectangular.
    pub fn sel_start(&mut self, col: u16, row: u16, block: bool) {
        self.core.sel_start(col, row, block);
    }

    /// Extend the active selection to viewport cell (col, row).
    pub fn sel_update(&mut self, col: u16, row: u16) {
        self.core.sel_update(col, row);
    }

    /// Select the word under viewport cell (col, row) — a double-click.
    pub fn sel_word(&mut self, col: u16, row: u16) {
        self.core.sel_word(col, row);
    }

    /// Select the whole logical line under viewport cell (col, row) — a
    /// triple-click; a soft-wrapped line comes back whole.
    pub fn sel_line(&mut self, col: u16, row: u16) {
        self.core.sel_line(col, row);
    }

    /// Clear any active selection.
    pub fn sel_clear(&mut self) {
        self.core.sel_clear();
    }

    /// The selected text, or `None` when nothing (non-empty) is selected.
    pub fn sel_text(&self) -> Option<String> {
        self.core.sel_text()
    }

    /// The program-set window title (OSC 0/2), empty if none.
    pub fn title(&self) -> String {
        self.core.title()
    }

    /// The directory the program reported via OSC 7 if it changed since the last
    /// call, else `None` — used to retitle the pane when the user `cd`s inside it.
    /// The OSC 133 semantic marks seen since the last call, oldest first.
    /// Empty for every shell with no integration configured, which is the
    /// point: this is an upgrade, not a requirement.
    pub fn take_shell(&mut self) -> Vec<crate::osc::ShellMark> {
        self.core.take_shell()
    }

    pub fn take_cwd(&mut self) -> Option<std::path::PathBuf> {
        self.core.take_cwd()
    }

    /// The spawned shell's own PID (the PTY child) — session restore asks the
    /// OS for this process's live working directory at quit. May be stale
    /// after the child exits (portable-pty keeps the stored pid); callers
    /// treat an OS miss as "no cwd" and fall back.
    pub fn shell_pid(&self) -> Option<u32> {
        self.child.process_id()
    }

    /// PID of the foreground command running in this pane — the process group in
    /// control of the tty. `None` when the shell itself is at its prompt (so the
    /// pane is idle) or on a platform that doesn't expose it. Lets the title name
    /// the running program (e.g. `claude`, `codex`).
    pub fn foreground_pid(&self) -> Option<u32> {
        // `process_group_leader` is a Unix-only portable-pty API; Windows has no
        // tty foreground-process-group concept, so the pane is simply never
        // labelled with a running command there.
        #[cfg(unix)]
        {
            let fg = u32::try_from(self.master.process_group_leader()?).ok()?;
            // A shell waiting at its prompt is its own foreground group → idle.
            if Some(fg) == self.child.process_id() {
                return None;
            }
            Some(fg)
        }
        #[cfg(not(unix))]
        {
            None
        }
    }

    /// Take any pending OSC 52 clipboard-store text (clearing it).
    pub fn take_clipboard(&self) -> Option<String> {
        self.core.take_clipboard()
    }

    /// Take a pending bell (rung since the last check), clearing it.
    /// A notification the program in this pane asked for (OSC 9 / OSC 777),
    /// once.
    pub fn take_notify(&mut self) -> Option<(String, String)> {
        self.core.take_notify()
    }

    /// What the program says about its own progress (OSC 9;4).
    pub fn progress(&self) -> Option<crate::osc::Progress> {
        self.core.progress()
    }

    /// The OSC 8 hyperlink target under viewport cell `(col, row)`.
    pub fn link_at(&self, col: u16, row: u16) -> Option<String> {
        self.core.link_at(col, row)
    }

    pub fn take_bell(&self) -> bool {
        self.core.take_bell()
    }
}

impl TermModel for PtyTerm {
    fn feed(&mut self, bytes: &[u8]) {
        self.core.feed(bytes);
    }

    fn cells(&self, focused: bool) -> Vec<RenderCell> {
        self.core.cells(focused)
    }

    fn last_line(&self) -> Option<String> {
        self.core.last_line()
    }

    fn wrapped_rows(&self) -> Vec<bool> {
        self.core.wrapped_rows()
    }

    fn resize(&mut self, size: GridSize) {
        self.core.resize(size);
        let _ = self.master.resize(self.core.pty_size());
    }
}

impl Drop for PtyTerm {
    /// Kill and reap the child explicitly — dropping the master only HUPs
    /// the child eventually, and a killed-but-unreaped child sits in the
    /// process table for the life of the app. The reap is a bounded poll:
    /// kill() already escalated to SIGKILL, so the child dies within
    /// milliseconds — but an unbounded wait() can wedge on a child that is
    /// itself blocked waiting on an untracked grandchild.
    fn drop(&mut self) {
        let _ = self.child.kill();
        for _ in 0..20 {
            match self.child.try_wait() {
                Ok(Some(_)) | Err(_) => return, // reaped (or gone)
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(5)),
            }
        }
        // Still not reaped after ~100ms of polling: give up rather than
        // hang the winit thread; the entry is reclaimed when the app exits.
        // (True worst case is ~300ms: kill() itself waits out a ~200ms
        // SIGHUP grace before escalating, on top of this poll loop.)
    }
}
