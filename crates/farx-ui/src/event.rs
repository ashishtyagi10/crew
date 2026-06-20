use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;

/// Events produced by the terminal for the application to consume.
#[derive(Debug)]
pub enum Event {
    /// A key press/release event.
    Key(KeyEvent),
    /// A mouse event (click, scroll, etc.).
    Mouse(MouseEvent),
    /// Terminal resize event with new (width, height).
    Resize(u16, u16),
    /// An embedded terminal produced output and the screen should redraw.
    /// Sent from PTY reader threads so output is shown immediately instead
    /// of waiting for the next periodic tick.
    TerminalOutput,
    /// Periodic tick for background updates.
    Tick,
}

/// Async event handler that bridges crossterm terminal events into a tokio
/// channel, combining them with periodic tick events.
pub struct EventHandler {
    tx: mpsc::UnboundedSender<Event>,
    rx: mpsc::UnboundedReceiver<Event>,
    _task: tokio::task::JoinHandle<()>,
}

impl EventHandler {
    /// Create a new EventHandler that polls terminal events and emits ticks
    /// at the given rate.
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let task_tx = tx.clone();
        let task = tokio::spawn(async move {
            let tx = task_tx;
            let mut reader = event::EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_rate);
            loop {
                tokio::select! {
                    maybe_event = reader.next() => {
                        match maybe_event {
                            Some(Ok(evt)) => {
                                match evt {
                                    CrosstermEvent::Key(key) => {
                                        let _ = tx.send(Event::Key(key));
                                    }
                                    CrosstermEvent::Mouse(mouse) => {
                                        let _ = tx.send(Event::Mouse(mouse));
                                    }
                                    CrosstermEvent::Resize(w, h) => {
                                        let _ = tx.send(Event::Resize(w, h));
                                    }
                                    _ => {}
                                }
                            }
                            Some(Err(_)) => {}
                            None => break,
                        }
                    }
                    _ = tick_interval.tick() => {
                        if tx.send(Event::Tick).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        Self {
            tx,
            rx,
            _task: task,
        }
    }

    /// A clone of the sender, so background producers (e.g. PTY reader
    /// threads) can push events such as [`Event::TerminalOutput`] that wake
    /// the loop for an immediate redraw.
    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.tx.clone()
    }

    /// Wait for the next event. Returns `None` if the channel is closed.
    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
