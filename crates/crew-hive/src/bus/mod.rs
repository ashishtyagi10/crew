//! Non-blocking event bus: workers publish `HiveEvent`s; UI/telemetry
//! subscribe. Backed by `tokio::sync::broadcast` so a slow/absent subscriber
//! never blocks a worker.
mod event;
#[cfg(test)]
mod tests;

pub use event::{AgentId, HiveEvent};

use tokio::sync::broadcast;

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<HiveEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Best-effort publish. With no subscribers `send` returns `Err`; that is
    /// expected (headless runs) and intentionally ignored — never block work.
    pub fn publish(&self, ev: HiveEvent) {
        let _ = self.tx.send(ev);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<HiveEvent> {
        self.tx.subscribe()
    }
}
