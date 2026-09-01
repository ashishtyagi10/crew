//! Telegram: the first way into crew that is not this machine's keyboard.
//!
//! Chosen over iMessage and WhatsApp for one reason — the Bot API is a documented, stable HTTP
//! surface with a token the user creates themselves. No scraping a private database, no
//! unofficial bridge that gets an account banned.
//!
//! The channel is INERT WITHOUT A TOKEN. With none configured it registers, reports `ready() ==
//! false`, and never opens a socket. That is deliberate: an autonomous build cannot create a
//! BotFather token, so the code ships complete and switches on the moment a human pastes one in.
//!
//! ## Setting it up (the part only a person can do)
//! 1. Message `@BotFather` on Telegram, send `/newbot`, follow the prompts, copy the token.
//! 2. `export CREW_TELEGRAM_TOKEN=<token>`
//! 3. Message your new bot once, then set `CREW_TELEGRAM_CHATS=<your chat id>` — crew prints the
//!    id of any chat it turns away, so the first rejected message tells you what to put there.

use super::{Channel, Inbound};

pub(crate) mod allow;
pub(crate) mod api;

pub(crate) use allow::Allowlist;

/// One message as the Bot API reports it, reduced to what a channel needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Update {
    /// Monotonic id; the next poll must ask for `update_id + 1` or the same message arrives
    /// forever.
    pub update_id: i64,
    pub chat_id: i64,
    pub text: String,
}

/// The Bot API calls this channel makes. A trait so the pump is testable without a token, a
/// network, or a bot.
pub(crate) trait TelegramApi: Send {
    /// Long-poll for messages newer than `offset`.
    fn get_updates(&self, offset: i64) -> Result<Vec<Update>, String>;
    /// Send one message to a chat.
    fn send_message(&self, chat_id: i64, text: &str) -> Result<(), String>;
}

/// What one poll produced: the messages to act on, the new offset, and the chats turned away.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Pumped {
    pub inbound: Vec<Inbound>,
    pub offset: i64,
    /// Chat ids refused by the allowlist. Surfaced rather than dropped in silence — this is how
    /// the owner learns their own chat id, and how they notice a stranger knocking.
    pub refused: Vec<i64>,
}

/// One poll cycle, pure apart from the API call. `offset` in, `offset` out.
///
/// The offset must advance past EVERY update seen, including refused ones. Advancing only past
/// accepted messages means one stranger's message pins the offset and crew re-reads it, and
/// everything behind it, on every poll forever.
pub(crate) fn pump(
    api: &dyn TelegramApi,
    offset: i64,
    allow: &Allowlist,
) -> Result<Pumped, String> {
    let updates = api.get_updates(offset)?;
    let mut out = Pumped {
        offset,
        ..Pumped::default()
    };
    for u in updates {
        out.offset = out.offset.max(u.update_id + 1);
        if !allow.allows(u.chat_id) {
            out.refused.push(u.chat_id);
            continue;
        }
        if u.text.trim().is_empty() {
            continue;
        }
        out.inbound.push(Inbound {
            from: format!("telegram:{}", u.chat_id),
            text: u.text,
        });
    }
    Ok(out)
}

/// The chat id in a `telegram:<id>` address.
pub(crate) fn chat_id_of(addr: &str) -> Option<i64> {
    super::split_address(addr)
        .filter(|(kind, _)| *kind == "telegram")
        .and_then(|(_, rest)| rest.parse().ok())
}

/// The Telegram channel. Holds no thread of its own: the daemon drives [`Telegram::tick`], the
/// same way it drives everything else, so there is one place that decides how often crew reaches
/// out and one place that can stop it.
pub(crate) struct Telegram {
    api: Option<std::sync::Arc<dyn TelegramApi + Sync>>,
    allow: Allowlist,
    /// Filled by the poll thread (when there is one), drained by `poll`.
    shared: std::sync::Arc<std::sync::Mutex<Shared>>,
    /// Whether the long-poll thread is running. Started once, on the first tick.
    started: bool,
}

/// What the poll thread and the channel share.
#[derive(Default)]
pub(crate) struct Shared {
    pub inbox: Vec<Inbound>,
    /// Chat ids refused since the last drain — how the owner learns their own id, and how they
    /// notice a stranger knocking.
    pub refused: Vec<i64>,
    pub offset: i64,
    /// Last transport error, so a channel that stopped receiving can say so instead of just
    /// going quiet.
    pub error: Option<String>,
}

impl Telegram {
    /// Build from the environment. No token means a channel that exists and does nothing.
    pub(crate) fn from_env() -> Self {
        let token = std::env::var("CREW_TELEGRAM_TOKEN")
            .ok()
            .filter(|t| !t.trim().is_empty());
        let allow = Allowlist::parse(&std::env::var("CREW_TELEGRAM_CHATS").unwrap_or_default());
        Self {
            api: token.map(|t| {
                std::sync::Arc::new(api::HttpApi::new(t)) as std::sync::Arc<dyn TelegramApi + Sync>
            }),
            allow,
            shared: Default::default(),
            started: false,
        }
    }

    /// With an explicit API — the test seam, and how a future config file will inject one.
    #[cfg(test)]
    pub(crate) fn with_api(api: std::sync::Arc<dyn TelegramApi + Sync>, allow: Allowlist) -> Self {
        Self {
            api: Some(api),
            allow,
            shared: Default::default(),
            started: false,
        }
    }

    /// Notices waiting, taken in one go.
    pub(crate) fn drain_notices(&mut self) -> (Vec<Inbound>, Vec<i64>, Option<String>) {
        let mut g = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        (
            std::mem::take(&mut g.inbox),
            std::mem::take(&mut g.refused),
            g.error.take(),
        )
    }

    /// One poll cycle, on the calling thread. The poll thread runs [`pump`] directly; this is
    /// how a test drives a cycle without one.
    #[cfg(test)]
    pub(crate) fn tick(&mut self) -> Result<(), String> {
        let Some(api) = self.api.clone() else {
            return Ok(());
        };
        let offset = self.shared.lock().unwrap_or_else(|e| e.into_inner()).offset;
        let p = pump(api.as_ref(), offset, &self.allow)?;
        let mut g = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        g.offset = p.offset;
        g.inbox.extend(p.inbound);
        g.refused.extend(p.refused);
        Ok(())
    }

    /// Start the long-poll thread, once. The Bot API holds a request open for 25 seconds, so
    /// this must never happen on the daemon's serve loop — a resident that stops answering for
    /// half a minute at a time is not a resident.
    ///
    /// A transport error is recorded rather than logged and forgotten, and the loop backs off
    /// instead of hammering a server that is refusing it.
    pub(crate) fn start(&mut self) {
        if self.started {
            return;
        }
        let Some(api) = self.api.clone() else {
            return;
        };
        self.started = true;
        let shared = std::sync::Arc::clone(&self.shared);
        let allow = self.allow.clone();
        std::thread::spawn(move || loop {
            let offset = shared.lock().unwrap_or_else(|e| e.into_inner()).offset;
            match pump(api.as_ref(), offset, &allow) {
                Ok(p) => {
                    let mut g = shared.lock().unwrap_or_else(|e| e.into_inner());
                    g.offset = p.offset;
                    g.inbox.extend(p.inbound);
                    g.refused.extend(p.refused);
                    g.error = None;
                }
                Err(e) => {
                    shared.lock().unwrap_or_else(|err| err.into_inner()).error = Some(e);
                    std::thread::sleep(std::time::Duration::from_secs(5));
                }
            }
        });
    }
}

impl Channel for Telegram {
    fn kind(&self) -> &str {
        "telegram"
    }

    fn poll(&mut self) -> Vec<Inbound> {
        // Starting here means the socket opens the first time the daemon actually looks for
        // messages, not when the channel is constructed — a crew with a token but no running
        // daemon still talks to nobody.
        self.start();
        std::mem::take(&mut self.shared.lock().unwrap_or_else(|e| e.into_inner()).inbox)
    }

    /// A refused chat id is the owner's own id, the first time they message their bot — so it
    /// is reported, not dropped. It is also how they notice a stranger knocking.
    fn notices(&mut self) -> Vec<String> {
        let (_, refused, error) = self.drain_notices();
        let mut out: Vec<String> = refused
            .into_iter()
            .map(|id| {
                format!(
                    "telegram: ignored a message from chat {id} \u{2014} add it to \
                     CREW_TELEGRAM_CHATS to let it through"
                )
            })
            .collect();
        if let Some(e) = error {
            out.push(format!("telegram: {e}"));
        }
        out
    }

    fn send(&mut self, to: &str, text: &str) -> Result<(), String> {
        let Some(api) = self.api.clone() else {
            return Err("no Telegram token configured (CREW_TELEGRAM_TOKEN)".into());
        };
        let Some(chat) = chat_id_of(to) else {
            return Err(format!("{to:?} is not a telegram address"));
        };
        // Crew must not answer a chat it would refuse to listen to: a reply is itself an
        // outbound message to a stranger, and the allowlist is about who crew talks to at all.
        if !self.allow.allows(chat) {
            return Err(format!("chat {chat} is not in CREW_TELEGRAM_CHATS"));
        }
        api.send_message(chat, text)
    }

    /// Configured means a token AND somebody to talk to. A bot with an empty allowlist can
    /// neither be addressed nor answer, so calling it ready would be a lie.
    fn ready(&self) -> bool {
        self.api.is_some() && !self.allow.is_empty()
    }

    /// A bot allowed to talk to exactly one chat has exactly one address.
    fn default_address(&self) -> Option<String> {
        self.allow.sole().map(|id| format!("telegram:{id}"))
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
