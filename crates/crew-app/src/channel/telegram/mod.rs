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
use std::collections::BTreeSet;

use super::{Channel, Inbound};

pub(crate) mod api;

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

/// Who crew will listen to. An assistant with a public address is an assistant anyone can drive,
/// so silence toward strangers is the default: an empty allowlist accepts NOBODY rather than
/// everybody. The alternative — open until configured — is a window that stands open for
/// exactly as long as it takes the owner to notice.
#[derive(Debug, Default, Clone)]
pub(crate) struct Allowlist(BTreeSet<i64>);

impl Allowlist {
    /// Parse `CREW_TELEGRAM_CHATS`: comma or space separated chat ids.
    pub(crate) fn parse(raw: &str) -> Self {
        Self(
            raw.split([',', ' ', '\t'])
                .filter_map(|s| s.trim().parse::<i64>().ok())
                .collect(),
        )
    }

    pub(crate) fn allows(&self, chat_id: i64) -> bool {
        self.0.contains(&chat_id)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
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
    api: Option<Box<dyn TelegramApi>>,
    allow: Allowlist,
    offset: i64,
    inbox: Vec<Inbound>,
    /// Chat ids refused since the last report, so the daemon can log them once. Read by 2.4,
    /// which is what turns "a stranger knocked" into a line the owner sees.
    #[allow(dead_code)]
    pub(crate) refused: Vec<i64>,
}

impl Telegram {
    /// Build from the environment. No token means a channel that exists and does nothing.
    pub(crate) fn from_env() -> Self {
        let token = std::env::var("CREW_TELEGRAM_TOKEN")
            .ok()
            .filter(|t| !t.trim().is_empty());
        let allow = Allowlist::parse(&std::env::var("CREW_TELEGRAM_CHATS").unwrap_or_default());
        Self {
            api: token.map(|t| Box::new(api::HttpApi::new(t)) as Box<dyn TelegramApi>),
            allow,
            offset: 0,
            inbox: Vec::new(),
            refused: Vec::new(),
        }
    }

    /// With an explicit API — the test seam, and how a future config file will inject one.
    #[cfg(test)]
    pub(crate) fn with_api(api: Box<dyn TelegramApi>, allow: Allowlist) -> Self {
        Self {
            api: Some(api),
            allow,
            offset: 0,
            inbox: Vec::new(),
            refused: Vec::new(),
        }
    }

    /// Fetch whatever has arrived. Driven by 2.4, which gives the daemon a worker thread to
    /// long-poll on — the Bot API holds a request open for 25s, so this must never sit on the
    /// serve loop.
    #[allow(dead_code)]
    ///
    /// Fetch whatever has arrived. A transport error is returned, never swallowed: a channel
    /// that quietly stops receiving is worse than one that says it is broken.
    pub(crate) fn tick(&mut self) -> Result<(), String> {
        let Some(api) = self.api.as_deref() else {
            return Ok(());
        };
        let p = pump(api, self.offset, &self.allow)?;
        self.offset = p.offset;
        self.inbox.extend(p.inbound);
        self.refused.extend(p.refused);
        Ok(())
    }
}

impl Channel for Telegram {
    fn kind(&self) -> &str {
        "telegram"
    }

    fn poll(&mut self) -> Vec<Inbound> {
        std::mem::take(&mut self.inbox)
    }

    fn send(&mut self, to: &str, text: &str) -> Result<(), String> {
        let Some(api) = self.api.as_deref() else {
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
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
