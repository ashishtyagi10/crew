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

    /// Lines currently scrolled back from the live bottom (0 = at the bottom).
    pub fn display_offset(&self) -> usize {
        self.core.display_offset()
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

    fn resize(&mut self, size: GridSize) {
        self.core.resize(size);
        let _ = self.master.resize(PtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        });
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
