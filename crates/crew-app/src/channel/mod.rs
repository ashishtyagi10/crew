//! Channels: the ways a request reaches crew and an answer gets back.
//!
//! A pane, a phone and a microphone are the same kind of thing — somewhere a request arrives
//! from and a reply leaves to. Written once as a trait, voice stops being a special subsystem
//! and becomes the third implementation of an interface Telegram already forced into existence.
//!
//! Addressing reuses the shape the sentinel work settled on (`docs/vision/sentinel-network.md`):
//! an address is `kind:rest`, opaque to everything except the channel that owns `kind`. The
//! router widens what an address can name; nothing downstream changes.
use std::collections::BTreeMap;

pub(crate) mod loopback;
pub(crate) mod telegram;

/// One message that arrived from the outside world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Inbound {
    /// Full address of the sender, `kind:rest` — the address a reply goes back to.
    pub from: String,
    /// What they said.
    pub text: String,
}

/// A way in and a way out. Implementations own their transport and nothing else: no sessions, no
/// policy, no idea what a broker is.
pub(crate) trait Channel: Send {
    /// The address kind this channel owns — the part before the first `:`.
    fn kind(&self) -> &str;
    /// Take everything that has arrived since the last call. Never blocks.
    /// Called by [`Router::poll`], which 2.2 puts on the daemon's serve loop.
    #[allow(dead_code)]
    fn poll(&mut self) -> Vec<Inbound>;
    /// Send `text` to a full `kind:rest` address on this channel.
    fn send(&mut self, to: &str, text: &str) -> Result<(), String>;
    /// Anything the operator should see: a refused sender, a transport error. Drained, so each
    /// notice is reported once.
    fn notices(&mut self) -> Vec<String> {
        Vec::new()
    }

    /// Is this channel usable right now? A channel with no credential configured is present but
    /// inert — it reports false rather than pretending, so `crew daemon status` can say which
    /// ways in are actually open.
    fn ready(&self) -> bool {
        true
    }
}

/// Split `kind:rest`. `None` when there is no kind — an address with no channel cannot be
/// delivered to, and guessing a default would send someone's reply to a stranger.
pub(crate) fn split_address(addr: &str) -> Option<(&str, &str)> {
    let (kind, rest) = addr.split_once(':')?;
    if kind.is_empty() || rest.is_empty() {
        return None;
    }
    Some((kind, rest))
}

/// Every registered channel, and the routing between them.
#[derive(Default)]
pub(crate) struct Router {
    channels: BTreeMap<String, Box<dyn Channel>>,
}

impl Router {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Register a channel. Refuses a second channel claiming the same kind: two owners of
    /// `telegram:` would make every reply a coin flip about which one delivers it.
    pub(crate) fn add(&mut self, c: Box<dyn Channel>) -> Result<(), String> {
        let kind = c.kind().to_string();
        if kind.is_empty() || kind.contains(':') {
            return Err(format!("{kind:?} is not a usable address kind"));
        }
        if self.channels.contains_key(&kind) {
            return Err(format!("a channel already owns the address kind {kind:?}"));
        }
        self.channels.insert(kind, c);
        Ok(())
    }

    /// The registered kinds, in a stable order.
    pub(crate) fn kinds(&self) -> Vec<&str> {
        self.channels.keys().map(String::as_str).collect()
    }

    /// The kinds that are actually usable — configured, connected, credential present.
    pub(crate) fn ready_kinds(&self) -> Vec<&str> {
        self.channels
            .iter()
            .filter(|(_, c)| c.ready())
            .map(|(k, _)| k.as_str())
            .collect()
    }

    /// Drain every channel. Each message keeps its full `from` address, so a reply always has
    /// somewhere to go — a message whose origin is lost is one nobody can answer.
    ///
    pub(crate) fn poll(&mut self) -> Vec<Inbound> {
        let mut out = Vec::new();
        for c in self.channels.values_mut() {
            out.extend(c.poll());
        }
        out
    }

    /// Send `text` to `addr`. An unroutable address is an error, never a silent drop: a reply
    /// nobody receives looks exactly like a reply that was never written.
    /// Every channel's notices, drained.
    pub(crate) fn notices(&mut self) -> Vec<String> {
        self.channels
            .values_mut()
            .flat_map(|c| c.notices())
            .collect()
    }

    pub(crate) fn send(&mut self, addr: &str, text: &str) -> Result<(), String> {
        let Some((kind, _)) = split_address(addr) else {
            return Err(format!(
                "{addr:?} has no channel — an address is kind:rest, like telegram:12345"
            ));
        };
        let Some(c) = self.channels.get_mut(kind) else {
            return Err(format!("no channel is registered for {kind:?}"));
        };
        if !c.ready() {
            return Err(format!("the {kind:?} channel is not configured"));
        }
        c.send(addr, text)
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
