//! A channel that goes nowhere: messages are pushed in by hand and sent messages pile up in an
//! outbox. It is how the router, the gate's approval round trip, and eventually the session
//! plumbing get tested without a network, a token, or a microphone.
use std::sync::{Arc, Mutex};

use super::{Channel, Inbound};

/// The shared state, so a test can drive one end while the router holds the other.
#[derive(Default)]
pub(crate) struct Wire {
    pub inbox: Vec<Inbound>,
    pub outbox: Vec<(String, String)>,
    pub ready: bool,
}

/// A [`Channel`] over a [`Wire`].
pub(crate) struct Loopback {
    kind: String,
    wire: Arc<Mutex<Wire>>,
}

impl Loopback {
    /// A ready loopback channel and the wire to drive it with. Used by tests today; it is also
    /// what the approval round trip will be exercised against before Telegram exists.
    #[allow(dead_code)]
    pub(crate) fn pair(kind: &str) -> (Self, Arc<Mutex<Wire>>) {
        let wire = Arc::new(Mutex::new(Wire {
            ready: true,
            ..Wire::default()
        }));
        (
            Self {
                kind: kind.to_string(),
                wire: Arc::clone(&wire),
            },
            wire,
        )
    }
}

impl Channel for Loopback {
    fn kind(&self) -> &str {
        &self.kind
    }

    fn poll(&mut self) -> Vec<Inbound> {
        std::mem::take(&mut self.wire.lock().unwrap_or_else(|e| e.into_inner()).inbox)
    }

    fn send(&mut self, to: &str, text: &str) -> Result<(), String> {
        self.wire
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .outbox
            .push((to.to_string(), text.to_string()));
        Ok(())
    }

    fn ready(&self) -> bool {
        self.wire.lock().unwrap_or_else(|e| e.into_inner()).ready
    }
}
