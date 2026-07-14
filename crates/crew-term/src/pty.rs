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
    /// Inter-pane `ask` output tap: `Some(buf)` while an ask targets this
    /// pane — `try_read` appends each raw output chunk (lossy UTF-8) so the
    /// asker's liveness engine can scan for its answer sentinel. `None` = off
    /// (zero overhead). See crew-app `askwait`.
    capture: Option<String>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl PtyTerm {
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
                    if let Some(buf) = &mut self.capture {
                        buf.push_str(&String::from_utf8_lossy(&chunk));
                    }
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

    /// Begin taking a copy of this pane's raw output for an inter-pane `ask`
    /// (see `take_capture`). Idempotent; keeps any already-captured bytes.
    pub fn start_capture(&mut self) {
        self.capture.get_or_insert_with(String::new);
    }

    /// Stop copying output (the ask resolved). Discards the buffer.
    pub fn stop_capture(&mut self) {
        self.capture = None;
    }

    /// Drain the output captured since the last call (empty when not
    /// capturing), leaving capture on so the next tick's delta accrues.
    pub fn take_capture(&mut self) -> String {
        self.capture
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default()
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

#[path = "ptyscan.rs"]
mod ptyscan;
#[path = "ptyspawn.rs"]
mod ptyspawn;
#[path = "ptyview.rs"]
mod ptyview;
use ptyscan::scan;
#[cfg(test)]
use ptyscan::strip_ansi;

#[cfg(test)]
#[path = "pty_tests.rs"]
mod pty_tests;
