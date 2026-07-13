use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::path::Path;
use std::sync::mpsc::{sync_channel, Receiver};

use crate::model::{GridSize, RenderCell, TermCore, TermModel};

/// Upper bound on chunks buffered from the reader thread. At 8 KiB per chunk
/// this caps buffered PTY output at ~8 MiB and applies backpressure to a runaway
/// program: once the OS pipe buffer and this queue fill, the child's `write`
/// blocks, throttling it to our drain rate instead of piling up unbounded
/// memory and unbounded parse work.
const CHANNEL_CAP: usize = 1024;

/// Maximum bytes drained from the PTY into the parser per `try_read` — i.e. per
/// poll tick. Without this cap a program that floods output (`yes`, `cat` of a
/// huge file, a noisy build) makes a single `try_read` parse the entire backlog
/// synchronously on the main thread, freezing rendering and input in EVERY pane
/// until it finishes. Capping per tick keeps the UI responsive; any remainder is
/// consumed on following ticks (see `has_pending`).
const READ_BUDGET: usize = 256 * 1024;

pub struct PtyTerm {
    core: TermCore,
    master: Box<dyn portable_pty::MasterPty + Send>,
    /// The single pty writer, shared between the app's input path (see
    /// [`PtyTerm::writer`]) and `try_read`'s query replies (OSC color / DSR
    /// answers) — portable-pty only hands out one writer per master.
    input: std::sync::Arc<std::sync::Mutex<Box<dyn std::io::Write + Send>>>,
    rx: Receiver<Vec<u8>>,
    exited: bool,
    /// Set by `try_read` when it stopped at `READ_BUDGET` with bytes still
    /// queued, so the caller can keep the poll loop hot until the backlog drains.
    pending: bool,
    /// Case-insensitive substrings watched in the output stream (lowercased).
    /// Empty disables scanning entirely (zero overhead).
    watch: Vec<String>,
    /// Trailing partial line carried between `try_read`s so a watched pattern
    /// split across reads still matches.
    scan_tail: String,
    /// Watched patterns matched since the last `take_matches`.
    hits: Vec<String>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl PtyTerm {
    /// Spawn a shell (no extra args).  Delegates to `spawn_args`.
    pub fn spawn(size: GridSize, shell: &str) -> anyhow::Result<Self> {
        Self::spawn_args(size, shell, &[])
    }

    /// Spawn `command` with `args` in a new PTY of the given size.
    pub fn spawn_args(size: GridSize, command: &str, args: &[String]) -> anyhow::Result<Self> {
        Self::spawn_in(size, command, args, None)
    }

    /// Spawn `command` with `args` in a new PTY, starting in `cwd` when given
    /// (otherwise the child inherits the host process's working directory).
    pub fn spawn_in(
        size: GridSize,
        command: &str,
        args: &[String],
        cwd: Option<&Path>,
    ) -> anyhow::Result<Self> {
        Self::spawn_with_env(size, command, args, cwd, &[])
    }

    /// As [`Self::spawn_in`], additionally setting `env` vars on the child —
    /// the host's env is inherited otherwise. Crew uses this to hand run panes
    /// the user's login-shell PATH: a Dock-launched app only inherits launchd's
    /// minimal one, under which almost no user command resolves.
    pub fn spawn_with_env(
        size: GridSize,
        command: &str,
        args: &[String],
        cwd: Option<&Path>,
        env: &[(&str, &str)],
    ) -> anyhow::Result<Self> {
        let pty = native_pty_system();
        let pair = pty.openpty(PtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut cmd = CommandBuilder::new(command);
        cmd.args(args);
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }
        // Advertise a capable terminal so TUI programs behave (env is otherwise
        // inherited from the host process, so $HOME/$PATH etc. are present).
        cmd.env("TERM", "xterm-256color");
        // Light/dark hint for programs that read $COLORFGBG instead of (or as
        // a fallback to) querying OSC 11 — agent CLIs pick their palette from
        // it, so it must match the theme active at spawn time.
        cmd.env(
            "COLORFGBG",
            crate::contrast::colorfgbg_for(crew_theme::theme().term_bg),
        );
        for (k, v) in env {
            cmd.env(k, v);
        }
        let child = pair.slave.spawn_command(cmd)?;
        // Drop the slave end so EOF propagates when the child exits.
        drop(pair.slave);

        // Spawn a reader thread: portable-pty reads are blocking. The channel is
        // bounded so a flooding child can't pile up unbounded output in memory —
        // a full queue blocks the reader (and in turn the child) until the main
        // thread drains it.
        let mut reader = pair.master.try_clone_reader()?;
        let (tx, rx) = sync_channel::<Vec<u8>>(CHANNEL_CAP);
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match std::io::Read::read(&mut reader, &mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        let input = pair.master.take_writer()?;
        Ok(Self {
            core: TermCore::new(size),
            master: pair.master,
            input: std::sync::Arc::new(std::sync::Mutex::new(input)),
            rx,
            exited: false,
            pending: false,
            watch: Vec::new(),
            scan_tail: String::new(),
            hits: Vec::new(),
            child,
        })
    }

    /// True once the child process has exited and all its output has been drained
    /// (the reader thread ended and the channel disconnected). Set by `try_read`.
    pub fn exited(&self) -> bool {
        self.exited
    }

    /// Returns a fresh writer to the master PTY end (sends input to the shell).
    /// Handles share one underlying writer, so this can be called repeatedly.
    pub fn writer(&self) -> Box<dyn std::io::Write + Send> {
        Box::new(SharedWriter(std::sync::Arc::clone(&self.input)))
    }

    /// Drains pending bytes from the reader thread into the terminal model,
    /// returning the number of bytes consumed this tick. At most `READ_BUDGET`
    /// bytes are drained per call so one flooding pane can't stall the event
    /// loop; when bytes remain queued past the budget, `has_pending` returns true
    /// and the rest is consumed on the next tick.
    pub fn try_read(&mut self) -> usize {
        use std::sync::mpsc::TryRecvError;
        let mut total = 0;
        self.pending = false;
        loop {
            // Stop once this tick's budget is spent. The reader thread can refill
            // the channel as fast as we drain it (a flooding child), so without
            // this cap the loop never sees `Empty` and parses forever, hanging
            // the event loop. Leftover bytes are flagged via `pending`.
            if total >= READ_BUDGET {
                self.pending = true;
                break;
            }
            match self.rx.try_recv() {
                Ok(chunk) => {
                    total += chunk.len();
                    self.core.feed(&chunk);
                    if !self.watch.is_empty() {
                        for hit in scan(&mut self.scan_tail, &chunk, &self.watch) {
                            if !self.hits.contains(&hit) {
                                self.hits.push(hit);
                            }
                        }
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // Reader thread ended → child exited and output is drained.
                    self.exited = true;
                    break;
                }
            }
        }
        // Queries parsed above (OSC color probes, DSR) owe the child an
        // answer on its input; unanswered probes make agent CLIs assume a
        // dark background and mis-pick their output palette.
        if let Some(reply) = self.core.take_replies() {
            let mut w = self.input.lock().unwrap_or_else(|e| e.into_inner());
            let _ = w
                .write_all(reply.as_bytes())
                .and_then(|_| std::io::Write::flush(&mut *w));
        }
        total
    }

    /// True when the last `try_read` left bytes queued (it hit `READ_BUDGET`).
    /// The poll loop uses this to keep draining promptly rather than waiting a
    /// full tick, so flooded output catches up without ever blocking the UI.
    pub fn has_pending(&self) -> bool {
        self.pending
    }

    /// Set the case-insensitive substrings watched in this pane's output. Blank
    /// entries are dropped; an empty list disables scanning. Lowercased here so
    /// matching in `scan` is a plain `contains`.
    pub fn set_watch_patterns(&mut self, patterns: &[String]) {
        self.watch = patterns
            .iter()
            .filter(|p| !p.is_empty())
            .map(|p| p.to_lowercase())
            .collect();
    }

    /// Take the watched patterns matched since the last call (clearing them).
    pub fn take_matches(&mut self) -> Vec<String> {
        std::mem::take(&mut self.hits)
    }
}

/// A handle onto the pane's shared pty writer (see [`PtyTerm::writer`]).
struct SharedWriter(std::sync::Arc<std::sync::Mutex<Box<dyn std::io::Write + Send>>>);

impl std::io::Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).flush()
    }
}

/// Max bytes of partial (newline-free) output carried between [`scan`] calls, so
/// a pattern split across reads still matches without letting a newline-free
/// flood (e.g. a progress bar redrawing with `\r`) grow the carry unbounded.
const SCAN_CARRY_CAP: usize = 4096;

/// Scan a freshly-read `chunk` for any watched `patterns` (already lowercased,
/// matched case-insensitively). `tail` carries the trailing partial line between
/// calls so a pattern split across reads still matches. ANSI escape sequences are
/// stripped before matching. Returns the patterns that matched a line completed
/// by this chunk. Pure and bounded — safe to run on the main thread.
fn scan(tail: &mut String, chunk: &[u8], patterns: &[String]) -> Vec<String> {
    if patterns.is_empty() {
        return Vec::new();
    }
    tail.push_str(&strip_ansi(&String::from_utf8_lossy(chunk)));
    let mut hits = Vec::new();
    // Everything up to and including the last line break is a set of completed
    // lines — scan it, then carry only the trailing partial line.
    if let Some(idx) = tail.rfind(['\n', '\r']) {
        let rest = tail.split_off(idx + 1);
        scan_into(tail, patterns, &mut hits);
        *tail = rest;
    }
    // A newline-free flood must not grow the carry without bound.
    if tail.len() > SCAN_CARRY_CAP {
        scan_into(tail, patterns, &mut hits);
        let cut = tail.len() - SCAN_CARRY_CAP;
        let cut = (cut..=tail.len())
            .find(|&i| tail.is_char_boundary(i))
            .unwrap_or(tail.len());
        *tail = tail[cut..].to_string();
    }
    hits
}

/// Push every pattern present in `hay` (case-insensitive) into `hits`, once each.
fn scan_into(hay: &str, patterns: &[String], hits: &mut Vec<String>) {
    let lower = hay.to_lowercase();
    for p in patterns {
        if lower.contains(p) && !hits.contains(p) {
            hits.push(p.clone());
        }
    }
}

/// Strip ANSI escape sequences (CSI `ESC [ … final`, OSC `ESC ] … BEL/ST`, and
/// other two-char `ESC x` escapes) and stray C0 control bytes (keeping `\n \r
/// \t`), so pattern matching sees plain text rather than colour codes.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    // CSI runs until a final byte in 0x40..=0x7e (`@`..=`~`).
                    while let Some(&nc) = chars.peek() {
                        chars.next();
                        if ('@'..='~').contains(&nc) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    // OSC runs until BEL or ST (`ESC \`).
                    while let Some(&nc) = chars.peek() {
                        chars.next();
                        if nc == '\u{07}' {
                            break;
                        }
                        if nc == '\u{1b}' {
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                _ => {
                    chars.next();
                }
            }
            continue;
        }
        if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
            continue;
        }
        out.push(c);
    }
    out
}

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

#[cfg(test)]
mod pty_tests {
    use super::*;
    use std::io::Write;
    use std::time::{Duration, Instant};

    #[test]
    fn echo_roundtrips_through_pty() {
        let mut term = PtyTerm::spawn(GridSize { cols: 40, rows: 10 }, "sh").unwrap();
        let mut w = term.writer();
        // Echo a unique token, then read until it shows up on the grid.
        w.write_all(b"printf CREWOK\n").unwrap();
        w.flush().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut found = false;
        while Instant::now() < deadline {
            term.try_read();
            let line: String = {
                let mut cs: Vec<_> = term.cells(true);
                cs.sort_by_key(|c| (c.row, c.col));
                cs.iter().map(|c| c.c).collect()
            };
            if line.contains("CREWOK") {
                found = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(found, "expected CREWOK to appear on the terminal grid");
    }

    #[test]
    fn try_read_caps_bytes_per_tick_under_flood() {
        // A program that floods stdout: a single tick must not drain the whole
        // backlog, or it would block the event loop (and every other pane).
        let mut term = PtyTerm::spawn(GridSize { cols: 80, rows: 24 }, "sh").unwrap();
        let mut w = term.writer();
        w.write_all(b"yes crew-flood-line\n").unwrap();
        w.flush().unwrap();
        // Let the reader thread buffer well past one tick's budget.
        std::thread::sleep(Duration::from_millis(250));

        // The budget is checked between chunks, so the final 8 KiB reader chunk
        // can overshoot slightly — the point is the drain is *bounded* to roughly
        // the budget instead of consuming the whole flood (which would hang).
        let n = term.try_read();
        assert!(
            n <= READ_BUDGET + 8192,
            "one tick drained {n} bytes, far over the {READ_BUDGET}-byte budget"
        );
        assert!(
            term.has_pending(),
            "expected a backlog to remain after a budget-capped read"
        );

        // Stop `yes` so the child doesn't keep spinning after the test.
        let _ = w.write_all(&[0x03]); // Ctrl-C to the foreground process group
        let _ = w.flush();
    }

    fn pats(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn scan_matches_a_completed_line() {
        let mut tail = String::new();
        let hits = scan(&mut tail, b"Build succeeded\n", &pats(&["build succeeded"]));
        assert_eq!(hits, vec!["build succeeded".to_string()]);
    }

    #[test]
    fn scan_is_case_insensitive() {
        let mut tail = String::new();
        let hits = scan(&mut tail, b"ERROR: boom\n", &pats(&["error"]));
        assert_eq!(hits, vec!["error".to_string()]);
    }

    #[test]
    fn scan_ignores_ansi_color_codes() {
        let mut tail = String::new();
        // Red "error" wrapped in SGR codes, plus an OSC title set.
        let chunk = b"\x1b]0;title\x07\x1b[31merror\x1b[0m here\n";
        let hits = scan(&mut tail, chunk, &pats(&["error"]));
        assert_eq!(hits, vec!["error".to_string()]);
    }

    #[test]
    fn scan_matches_across_a_chunk_boundary() {
        let mut tail = String::new();
        // The pattern is split across two reads; no newline yet → no match.
        let first = scan(&mut tail, b"Build suc", &pats(&["build succeeded"]));
        assert!(first.is_empty());
        // The newline completes the line and the carried tail makes it match.
        let second = scan(&mut tail, b"ceeded\n", &pats(&["build succeeded"]));
        assert_eq!(second, vec!["build succeeded".to_string()]);
    }

    #[test]
    fn scan_does_not_rematch_an_already_consumed_line() {
        let mut tail = String::new();
        let first = scan(&mut tail, b"error here\n", &pats(&["error"]));
        assert_eq!(first, vec!["error".to_string()]);
        // A later read with no new match must not re-report the old line.
        let second = scan(&mut tail, b"all good\n", &pats(&["error"]));
        assert!(second.is_empty());
    }

    #[test]
    fn scan_empty_patterns_is_a_noop() {
        let mut tail = String::new();
        let hits = scan(&mut tail, b"anything at all\n", &[]);
        assert!(hits.is_empty());
        assert!(
            tail.is_empty(),
            "no work and no buffering when nothing is watched"
        );
    }

    #[test]
    fn scan_keeps_the_tail_bounded_under_a_newline_free_flood() {
        let mut tail = String::new();
        let pat = pats(&["needle"]);
        // 100 KiB with no newline must not grow the carry without bound.
        for _ in 0..1000 {
            scan(&mut tail, &[b'x'; 100], &pat);
        }
        assert!(tail.len() <= 4096, "carry grew to {} bytes", tail.len());
    }

    #[test]
    fn strip_ansi_removes_csi_and_osc() {
        assert_eq!(strip_ansi("\x1b[1;31mhi\x1b[0m"), "hi");
        assert_eq!(strip_ansi("\x1b]0;my title\x07done"), "done");
        assert_eq!(strip_ansi("plain"), "plain");
    }
}
