//! The output side of a background process: a ring of its last bytes that
//! reader threads fill and `sys:logs` / `wait_for` read (see `sysbg`).
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct Buf {
    pub(super) data: Vec<u8>,
    pub(super) dropped: usize,
    /// Pipes still open (stdout, stderr): the exit is only announced once the
    /// output is in, or `logs` right after an exit would miss the last line.
    pub(super) open: usize,
    pub(super) exit: Option<String>,
}

/// What the reader threads and the waiter share with the registry.
pub(super) struct Shared {
    pub(super) buf: Mutex<Buf>,
    pub(super) cv: Condvar,
    pub(super) cap: usize,
}

/// How a wait ended.
#[derive(Debug, PartialEq)]
pub(super) enum Waited {
    Found,
    Exited,
    TimedOut,
    /// The task was stopped (`/stop`) while waiting.
    Stopped,
}

impl Shared {
    pub(super) fn push(&self, chunk: &[u8]) {
        let mut b = self.buf.lock().unwrap_or_else(|e| e.into_inner());
        b.data.extend_from_slice(chunk);
        if b.data.len() > self.cap {
            let cut = b.data.len() - self.cap;
            b.data.drain(..cut);
            b.dropped += cut;
        }
        self.cv.notify_all();
    }

    /// Block until `needle` shows in the output (checked first, so a process
    /// that said it and then died counts as found), the process exits, `stop`
    /// is set, or `limit` passes. Sleeps in slices of at most 200 ms so a
    /// `/stop` is noticed promptly, and scans only what arrived since the last
    /// look (plus a needle-length overlap for a match split across chunks).
    pub(super) fn wait(&self, needle: Option<&str>, limit: Duration, stop: &AtomicBool) -> Waited {
        let end = Instant::now() + limit;
        let needle = needle.map(str::as_bytes);
        let mut seen = 0usize; // absolute bytes already searched
        let mut b = self.buf.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if let Some(n) = needle {
                let from = seen.saturating_sub(n.len().saturating_sub(1));
                let start = from.saturating_sub(b.dropped).min(b.data.len());
                if n.is_empty() || b.data[start..].windows(n.len()).any(|w| w == n) {
                    return Waited::Found;
                }
                seen = b.dropped + b.data.len();
            }
            if b.exit.is_some() {
                return Waited::Exited;
            }
            if stop.load(Ordering::SeqCst) {
                return Waited::Stopped;
            }
            let left = end.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Waited::TimedOut;
            }
            b = self
                .cv
                .wait_timeout(b, left.min(Duration::from_millis(200)))
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    /// `(text, bytes dropped from the front, exit if over)`.
    pub(super) fn read(&self) -> (String, usize, Option<String>) {
        let b = self.buf.lock().unwrap_or_else(|e| e.into_inner());
        (
            String::from_utf8_lossy(&b.data).into_owned(),
            b.dropped,
            b.exit.clone(),
        )
    }

    pub(super) fn running(&self) -> bool {
        self.buf
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .exit
            .is_none()
    }

    pub(super) fn reader(self: &Arc<Self>, mut pipe: impl Read + Send + 'static) {
        let me = Arc::clone(self);
        std::thread::spawn(move || {
            let mut chunk = [0u8; 8192];
            while let Ok(n) = pipe.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                me.push(&chunk[..n]);
            }
            let mut b = me.buf.lock().unwrap_or_else(|e| e.into_inner());
            b.open -= 1;
            me.cv.notify_all();
        });
    }
}
