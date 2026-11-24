//! A pane's input, written from its own thread.
//!
//! Writing to a pty blocks while the program on the other end isn't
//! reading: a raw-mode program that is busy or stuck (a hung TUI, ssh over
//! a stalled link) takes nothing, and the kernel's buffer is about a
//! kilobyte. Every key, paste and query reply was written on the window's
//! thread, so a Cmd+V into such a program froze every pane and the whole UI
//! until the program read or died (measured 2026-11-02: a 4 KiB write sat
//! 3.8 s on a `stty raw; sleep 4` child, then failed). Now each pane's
//! writes go into a queue a thread of its own drains, in order. A write
//! returns at once; once a write to the program has failed (it is gone),
//! the next one reports a broken pipe.
use std::io::{Error, ErrorKind, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;

/// The sending end of a pane's input queue. Clones share the queue, so
/// keys, pastes and replies reach the program in the order they were sent.
/// The count is the bytes sent but not yet taken by the program.
#[derive(Clone)]
pub(super) struct Input(Sender<Vec<u8>>, Arc<AtomicUsize>);

/// Start the thread that owns the pty's writer.
pub(super) fn start(mut writer: Box<dyn Write + Send>) -> Input {
    let (tx, rx) = channel::<Vec<u8>>();
    let backlog = Arc::new(AtomicUsize::new(0));
    let left = backlog.clone();
    std::thread::spawn(move || {
        for bytes in rx {
            let ok = writer.write_all(&bytes).and_then(|_| writer.flush());
            left.fetch_sub(bytes.len(), Ordering::Relaxed);
            if ok.is_err() {
                break;
            }
        }
    });
    Input(tx, backlog)
}

impl super::PtyTerm {
    /// Bytes written to this pane's program that it has not taken yet. A
    /// program that stops reading leaves them queued in crew's memory, so a
    /// caller about to send a large reply (an OSC 52 clipboard read) waits
    /// for this to drain rather than stack another copy behind it.
    pub fn input_backlog(&self) -> usize {
        self.input.1.load(Ordering::Relaxed)
    }
}

impl Write for Input {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Counted before the send, so the thread's decrement can't run first.
        self.1.fetch_add(buf.len(), Ordering::Relaxed);
        match self.0.send(buf.to_vec()) {
            Ok(()) => Ok(buf.len()),
            Err(_) => {
                self.1.fetch_sub(buf.len(), Ordering::Relaxed);
                Err(Error::new(
                    ErrorKind::BrokenPipe,
                    "the pane's program is gone",
                ))
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "ptywrite_tests.rs"]
mod tests;
