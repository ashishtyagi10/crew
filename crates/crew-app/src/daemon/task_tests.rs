//! Whether a message from a phone actually reaches an agent, and whether its answer comes back.
use super::*;
use crate::daemon::session::{SessionProc, Spawner};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// A stand-in broker: records what was written, replays canned output lines.
#[derive(Default)]
struct Recorder {
    written: Vec<String>,
    out: Vec<String>,
    alive: bool,
    /// The requester each spawn was told about — the value that decides whether the broker's
    /// gate trusts its caller.
    requesters: Vec<Option<String>>,
}

struct Fake(Arc<Mutex<Recorder>>);

impl SessionProc for Fake {
    fn alive(&mut self) -> bool {
        self.0.lock().unwrap().alive
    }
    fn kill(&mut self) {
        self.0.lock().unwrap().alive = false;
    }
    fn send(&mut self, line: &str) -> bool {
        let mut g = self.0.lock().unwrap();
        if !g.alive {
            return false;
        }
        g.written.push(line.to_string());
        true
    }
    fn output(&self) -> (Vec<String>, usize) {
        (self.0.lock().unwrap().out.clone(), 0)
    }
}

struct FakeSpawner(Arc<Mutex<Recorder>>);

impl Spawner for FakeSpawner {
    fn spawn(
        &mut self,
        _cwd: Option<&Path>,
        requester: Option<&str>,
    ) -> std::io::Result<Box<dyn SessionProc>> {
        let mut g = self.0.lock().unwrap();
        g.alive = true;
        g.requesters.push(requester.map(str::to_string));
        drop(g);
        Ok(Box::new(Fake(Arc::clone(&self.0))))
    }
}

fn rig() -> (Bridge, Registry, Arc<Mutex<Recorder>>) {
    let rec = Arc::new(Mutex::new(Recorder::default()));
    let reg = Registry::new(Box::new(FakeSpawner(Arc::clone(&rec))));
    (Bridge::default(), reg, rec)
}

fn message(text: &str) -> String {
    serde_json::to_string(&PluginEvent::Message {
        channel: CHANNEL.into(),
        sender: "crew".into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
    })
    .unwrap()
}

#[test]
fn a_message_opens_a_session_handshakes_and_sends_the_text() {
    let (mut b, mut reg, rec) = rig();
    assert_eq!(
        b.dispatch(&mut reg, "telegram:42", "run the tests"),
        Ok(ACK)
    );
    let g = rec.lock().unwrap();
    assert_eq!(
        g.written.len(),
        2,
        "a handshake and the message: {:?}",
        g.written
    );
    assert!(
        g.written[0].contains("hello"),
        "the broker is greeted first: {}",
        g.written[0]
    );
    assert!(
        g.written[1].contains("run the tests"),
        "and then told the task: {}",
        g.written[1]
    );
}

/// A conversation from a phone should remember the last thing you said, so the same address
/// keeps its session rather than getting a fresh, contextless broker per message.
#[test]
fn the_same_address_keeps_one_session() {
    let (mut b, mut reg, _rec) = rig();
    b.dispatch(&mut reg, "telegram:42", "first").unwrap();
    b.dispatch(&mut reg, "telegram:42", "second").unwrap();
    assert_eq!(b.len(), 1);
    assert_eq!(reg.len(), 1, "one broker, not two");
}

/// Two people (or two rooms) must not share a conversation.
#[test]
fn different_addresses_get_different_sessions() {
    let (mut b, mut reg, _rec) = rig();
    b.dispatch(&mut reg, "telegram:42", "mine").unwrap();
    b.dispatch(&mut reg, "voice:kitchen", "ours").unwrap();
    assert_eq!(b.len(), 2);
    assert_eq!(reg.len(), 2);
}

#[test]
fn a_brokers_message_comes_back_addressed_to_the_sender() {
    let (mut b, mut reg, rec) = rig();
    b.dispatch(&mut reg, "telegram:42", "what is 2+2").unwrap();
    rec.lock().unwrap().out.push(message("4"));
    let out = b.collect(&reg);
    assert_eq!(out, vec![("telegram:42".to_string(), "4".to_string())]);
}

/// The broker streams activity, stats, deltas and task lifecycle alongside the reply. Forwarding
/// those would turn a phone conversation into a debug log.
#[test]
fn only_finished_replies_are_forwarded() {
    let (mut b, mut reg, rec) = rig();
    b.dispatch(&mut reg, "telegram:42", "go").unwrap();
    {
        let mut g = rec.lock().unwrap();
        g.out.push(
            serde_json::to_string(&PluginEvent::Activity {
                agent: "planner".into(),
                state: "thinking".into(),
                from: "user".into(),
            })
            .unwrap(),
        );
        g.out.push(
            serde_json::to_string(&PluginEvent::StatsTick {
                agent: "planner".into(),
                tokens: 12,
            })
            .unwrap(),
        );
        g.out.push(message("done"));
        g.out.push("not json at all".to_string());
    }
    let out = b.collect(&reg);
    assert_eq!(out, vec![("telegram:42".to_string(), "done".to_string())]);
}

/// Collecting twice must not resend an answer — a phone buzzing twice for one reply is the
/// most visible bug this bridge could have.
#[test]
fn a_reply_is_delivered_once() {
    let (mut b, mut reg, rec) = rig();
    b.dispatch(&mut reg, "telegram:42", "go").unwrap();
    rec.lock().unwrap().out.push(message("done"));
    assert_eq!(b.collect(&reg).len(), 1);
    assert!(
        b.collect(&reg).is_empty(),
        "the second collection is silent"
    );
}

/// An empty reply is not worth a notification.
#[test]
fn an_empty_reply_is_not_forwarded() {
    let (mut b, mut reg, rec) = rig();
    b.dispatch(&mut reg, "telegram:42", "go").unwrap();
    rec.lock().unwrap().out.push(message("   "));
    assert!(b.collect(&reg).is_empty());
}

/// If the broker died, the sender must be told — and the next message must start fresh rather
/// than writing into a pipe that will never answer.
#[test]
fn a_dead_session_is_reported_and_forgotten() {
    let (mut b, mut reg, rec) = rig();
    b.dispatch(&mut reg, "telegram:42", "first").unwrap();
    rec.lock().unwrap().alive = false;
    let err = b.dispatch(&mut reg, "telegram:42", "second").unwrap_err();
    assert!(err.contains("stopped"), "{err}");
    assert_eq!(b.len(), 0, "the dead route is forgotten");
    // And the next message opens a new one.
    assert_eq!(b.dispatch(&mut reg, "telegram:42", "third"), Ok(ACK));
    assert_eq!(b.len(), 1);
}

/// The hole this closes: a broker started for a phone conversation used to report itself as a
/// person at the keyboard, so the gate inside it allowed irreversible calls that should have
/// needed approval. The session must be told who it is working for.
#[test]
fn a_session_opened_for_a_channel_tells_the_broker_it_is_not_a_pane() {
    let (mut b, mut reg, rec) = rig();
    b.dispatch(&mut reg, "telegram:42", "delete everything")
        .unwrap();
    let g = rec.lock().unwrap();
    assert_eq!(g.requesters.len(), 1);
    let raw = g.requesters[0].as_deref().expect("a requester was passed");
    let parsed = crew_plugin::approval::Requester::parse(raw);
    assert_eq!(
        parsed,
        crew_plugin::approval::Requester::Channel("telegram:42".into())
    );
    assert!(
        !parsed.is_present_human(),
        "a phone is not a person at the keyboard"
    );
}
