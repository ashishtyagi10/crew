//! App-wide activity log plumbing. Two pieces: the [`LogEntry`] level carried
//! by every sidebar LOG line (so errors render distinctly from routine
//! activity), and a channel any worker thread can clone a sender of to stream
//! lines into the LOG without touching the winit thread — the generalized
//! form of the `far_statuses` collect-then-flash pattern in `poll.rs`.
use std::sync::mpsc::{self, Receiver, Sender};

/// How loud a LOG line is: routine activity vs something that went wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Error,
}

/// One line of the sidebar LOG: its level and the (timestamped) text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub level: LogLevel,
    pub text: String,
}

/// The channel background threads report through. The receiver lives on the
/// app and is drained once per poll tick; senders are cloned into any thread
/// with something to say (relay listener, update worker, …).
pub struct AppLog {
    tx: Sender<(LogLevel, String)>,
    rx: Receiver<(LogLevel, String)>,
}

impl Default for AppLog {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx }
    }
}

impl AppLog {
    /// A cloneable sender for a background thread.
    pub fn sender(&self) -> Sender<(LogLevel, String)> {
        self.tx.clone()
    }

    /// Everything queued since the last drain (never blocks).
    pub fn drain(&self) -> Vec<(LogLevel, String)> {
        let mut out = Vec::new();
        while let Ok(item) = self.rx.try_recv() {
            out.push(item);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_returns_queued_items_in_order_then_empties() {
        let log = AppLog::default();
        log.sender().send((LogLevel::Info, "a".into())).unwrap();
        log.sender().send((LogLevel::Error, "b".into())).unwrap();
        let drained = log.drain();
        assert_eq!(
            drained,
            vec![
                (LogLevel::Info, "a".to_string()),
                (LogLevel::Error, "b".to_string())
            ]
        );
        assert!(log.drain().is_empty(), "a second drain finds nothing");
    }

    #[test]
    fn a_thread_can_send_through_a_cloned_sender() {
        let log = AppLog::default();
        let tx = log.sender();
        std::thread::spawn(move || {
            tx.send((LogLevel::Info, "from thread".into())).unwrap();
        })
        .join()
        .unwrap();
        assert_eq!(
            log.drain(),
            vec![(LogLevel::Info, "from thread".to_string())]
        );
    }
}
