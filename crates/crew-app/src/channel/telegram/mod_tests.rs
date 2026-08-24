//! The pump's invariants, the allowlist, and the states a channel with no token must be in.
use super::*;
use std::sync::Mutex;

/// Canned updates, recorded sends.
struct FakeApi {
    updates: Mutex<Vec<Vec<Update>>>,
    sent: Mutex<Vec<(i64, String)>>,
    fail: bool,
}

impl FakeApi {
    fn with(rounds: Vec<Vec<Update>>) -> Self {
        Self {
            updates: Mutex::new(rounds),
            sent: Mutex::new(Vec::new()),
            fail: false,
        }
    }
    fn broken() -> Self {
        Self {
            updates: Mutex::new(Vec::new()),
            sent: Mutex::new(Vec::new()),
            fail: true,
        }
    }
}

impl TelegramApi for FakeApi {
    fn get_updates(&self, _offset: i64) -> Result<Vec<Update>, String> {
        if self.fail {
            return Err("network is down".into());
        }
        let mut q = self.updates.lock().unwrap();
        if q.is_empty() {
            Ok(Vec::new())
        } else {
            Ok(q.remove(0))
        }
    }
    fn send_message(&self, chat_id: i64, text: &str) -> Result<(), String> {
        if self.fail {
            return Err("network is down".into());
        }
        self.sent.lock().unwrap().push((chat_id, text.to_string()));
        Ok(())
    }
}

fn upd(id: i64, chat: i64, text: &str) -> Update {
    Update {
        update_id: id,
        chat_id: chat,
        text: text.into(),
    }
}

fn allow(ids: &[i64]) -> Allowlist {
    Allowlist::parse(&ids.iter().map(i64::to_string).collect::<Vec<_>>().join(","))
}

#[test]
fn an_allowed_message_becomes_an_addressed_inbound() {
    let api = FakeApi::with(vec![vec![upd(5, 42, "what is on my calendar")]]);
    let p = pump(&api, 0, &allow(&[42])).unwrap();
    assert_eq!(p.inbound.len(), 1);
    assert_eq!(
        p.inbound[0].from, "telegram:42",
        "the reply address is the chat"
    );
    assert_eq!(p.inbound[0].text, "what is on my calendar");
}

/// The offset bug that makes a bot re-answer the same message forever.
#[test]
fn the_offset_advances_past_everything_seen() {
    let api = FakeApi::with(vec![vec![upd(7, 42, "a"), upd(9, 42, "b")]]);
    let p = pump(&api, 0, &allow(&[42])).unwrap();
    assert_eq!(p.offset, 10, "one past the highest update id");
}

/// …including refused ones. Advancing only past ACCEPTED messages means one stranger's message
/// pins the offset and crew re-reads it, and everything behind it, on every poll forever.
#[test]
fn a_refused_message_still_advances_the_offset() {
    let api = FakeApi::with(vec![vec![upd(3, 999, "hello from a stranger")]]);
    let p = pump(&api, 0, &allow(&[42])).unwrap();
    assert!(p.inbound.is_empty(), "the stranger is not acted on");
    assert_eq!(p.offset, 4, "but crew does not read it again forever");
    assert_eq!(p.refused, vec![999], "and the owner is told who knocked");
}

/// An assistant with a public address is an assistant anyone can drive. Empty means nobody.
#[test]
fn an_empty_allowlist_accepts_nobody_rather_than_everybody() {
    let api = FakeApi::with(vec![vec![upd(1, 42, "hi"), upd(2, 7, "hi")]]);
    let p = pump(&api, 0, &Allowlist::default()).unwrap();
    assert!(
        p.inbound.is_empty(),
        "an unconfigured bot must not take orders from the internet"
    );
    assert_eq!(p.refused.len(), 2);
}

#[test]
fn the_allowlist_parses_commas_and_spaces_and_ignores_junk() {
    let a = Allowlist::parse(" 42, 7 ,, nonsense 13 ");
    assert!(a.allows(42) && a.allows(7) && a.allows(13));
    assert!(!a.allows(1));
    assert!(Allowlist::parse("").is_empty());
}

#[test]
fn an_empty_message_is_not_an_inbound() {
    let api = FakeApi::with(vec![vec![upd(1, 42, "   ")]]);
    let p = pump(&api, 0, &allow(&[42])).unwrap();
    assert!(p.inbound.is_empty());
    assert_eq!(p.offset, 2, "still consumed");
}

/// A channel that quietly stops receiving is worse than one that says it is broken.
#[test]
fn a_transport_failure_is_reported_not_swallowed() {
    let api = FakeApi::broken();
    assert!(pump(&api, 0, &allow(&[42])).is_err());
    let mut t = Telegram::with_api(std::sync::Arc::new(FakeApi::broken()), allow(&[42]));
    assert!(t.tick().is_err());
}

#[test]
fn an_address_round_trips_through_the_chat_id() {
    assert_eq!(chat_id_of("telegram:4242"), Some(4242));
    assert_eq!(
        chat_id_of("telegram:-100123"),
        Some(-100123),
        "groups are negative"
    );
    assert_eq!(chat_id_of("voice:4242"), None, "another channel's address");
    assert_eq!(chat_id_of("telegram:not-a-number"), None);
}

/// Records what was sent, so a test can prove the message reached the right chat.
struct Probe(std::sync::Arc<Mutex<Vec<(i64, String)>>>);
impl TelegramApi for Probe {
    fn get_updates(&self, _o: i64) -> Result<Vec<Update>, String> {
        Ok(Vec::new())
    }
    fn send_message(&self, chat_id: i64, text: &str) -> Result<(), String> {
        self.0.lock().unwrap().push((chat_id, text.to_string()));
        Ok(())
    }
}

#[test]
fn sending_reaches_the_chat_the_address_names() {
    let log = std::sync::Arc::new(Mutex::new(Vec::new()));
    let mut t = Telegram::with_api(std::sync::Arc::new(Probe(log.clone())), allow(&[42]));
    t.send("telegram:42", "on it").unwrap();
    assert_eq!(log.lock().unwrap().as_slice(), &[(42, "on it".to_string())]);
}

/// Crew must not answer a chat it would refuse to listen to — a reply is itself an outbound
/// message to a stranger.
#[test]
fn crew_will_not_message_a_chat_outside_the_allowlist() {
    let mut t = Telegram::with_api(std::sync::Arc::new(FakeApi::with(vec![])), allow(&[42]));
    let err = t.send("telegram:999", "hello").unwrap_err();
    assert!(err.contains("999"), "{err}");
}

/// The state this ships in tonight: present, listed, and doing nothing at all.
#[test]
fn without_a_token_the_channel_is_inert() {
    // `from_env` with no token set — the shape a fresh install is in.
    let saved = std::env::var("CREW_TELEGRAM_TOKEN").ok();
    std::env::remove_var("CREW_TELEGRAM_TOKEN");
    let mut t = Telegram::from_env();
    assert!(!t.ready(), "not ready without a token");
    assert!(t.tick().is_ok(), "ticking is a no-op, not an error");
    assert!(t.poll().is_empty());
    let err = t.send("telegram:42", "hello").unwrap_err();
    assert!(
        err.contains("CREW_TELEGRAM_TOKEN"),
        "the error says what is missing: {err}"
    );
    if let Some(v) = saved {
        std::env::set_var("CREW_TELEGRAM_TOKEN", v);
    }
}

/// A token with nobody to talk to is not a working channel, and saying "ready" would be a lie.
#[test]
fn a_token_without_an_allowlist_is_not_ready() {
    let t = Telegram::with_api(
        std::sync::Arc::new(FakeApi::with(vec![])),
        Allowlist::default(),
    );
    assert!(!t.ready());
    let t = Telegram::with_api(std::sync::Arc::new(FakeApi::with(vec![])), allow(&[42]));
    assert!(t.ready());
}
