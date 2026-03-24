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
    /// Periodic tick for background updates.
    Tick,
}

/// Async event handler that bridges crossterm terminal events into a tokio
/// channel, combining them with periodic tick events.
pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    _task: tokio::task::JoinHandle<()>,
}

impl EventHandler {
    /// Create a new EventHandler that polls terminal events and emits ticks
    /// at the given rate.
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
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
        Self { rx, _task: task }
    }

    /// Wait for the next event. Returns `None` if the channel is closed.
    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
