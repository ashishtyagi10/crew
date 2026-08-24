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

// ---- approval over a channel ---------------------------------------------------------------

fn approval(id: &str) -> String {
    serde_json::to_string(&PluginEvent::Approval {
        id: id.into(),
        tool: "sys:run".into(),
        tier: "irreversible".into(),
        reply_to: "telegram:42".into(),
        question: "sys:run is about to run and cannot be undone. Reply yes to allow, no to refuse."
            .into(),
    })
    .unwrap()
}

/// A yes or a no, in the words people actually use.
#[test]
fn an_answer_is_read_from_ordinary_words() {
    for yes in [
        "yes", "Yes", " y ", "ok", "approve", "go ahead", "do it", "sure.",
    ] {
        assert_eq!(parse_answer(yes), Some(true), "{yes:?} is a yes");
    }
    for no in ["no", "N", "nope", "deny", "stop", "cancel", "don't"] {
        assert_eq!(parse_answer(no), Some(false), "{no:?} is a no");
    }
}

/// The whole point of asking is that somebody meant to say yes or no. Reading "maybe later" as
/// either is worse than asking twice.
#[test]
fn anything_that_is_not_a_yes_or_a_no_is_neither() {
    for unclear in [
        "maybe",
        "later",
        "",
        "what?",
        "yes but only the first one",
        "hmm",
    ] {
        assert_eq!(parse_answer(unclear), None, "{unclear:?} is not an answer");
    }
}

/// The question reaches the sender, and the bridge remembers what it is an answer to.
#[test]
fn an_approval_is_carried_to_the_sender() {
    let (mut b, mut reg, rec) = rig();
    b.dispatch(&mut reg, "telegram:42", "clean up the branch")
        .unwrap();
    rec.lock().unwrap().out.push(approval("a1"));
    let out = b.collect(&reg);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].0, "telegram:42", "asked where the work came from");
    assert!(
        out[0].1.contains("sys:run"),
        "and it names the tool: {}",
        out[0].1
    );
}

/// Saying yes sends the answer to the broker that is blocked on it.
#[test]
fn a_yes_answers_the_approval_rather_than_starting_new_work() {
    let (mut b, mut reg, rec) = rig();
    b.dispatch(&mut reg, "telegram:42", "clean up the branch")
        .unwrap();
    rec.lock().unwrap().out.push(approval("a1"));
    b.collect(&reg);
    let before = rec.lock().unwrap().written.len();
    assert_eq!(b.dispatch(&mut reg, "telegram:42", "yes"), Ok(ALLOWED));
    let g = rec.lock().unwrap();
    assert_eq!(g.written.len(), before + 1, "one command, not a new task");
    let sent = g.written.last().unwrap();
    assert!(sent.contains("approve"), "it is an approval answer: {sent}");
    assert!(sent.contains("a1"), "for the right approval: {sent}");
    assert!(sent.contains("true"), "and it is a yes: {sent}");
    assert!(
        !sent.contains("clean up"),
        "the word 'yes' did not become a new task: {sent}"
    );
}

#[test]
fn a_no_refuses_it() {
    let (mut b, mut reg, rec) = rig();
    b.dispatch(&mut reg, "telegram:42", "delete the repo")
        .unwrap();
    rec.lock().unwrap().out.push(approval("a2"));
    b.collect(&reg);
    assert_eq!(b.dispatch(&mut reg, "telegram:42", "no"), Ok(REFUSED));
    let g = rec.lock().unwrap();
    let sent = g.written.last().unwrap();
    assert!(sent.contains("false"), "a refusal is sent as one: {sent}");
}

/// An unclear reply must not be guessed at, and must not clear the question either — the agent
/// is still blocked, so the next thing said is still an answer.
#[test]
fn an_unclear_answer_asks_again_and_keeps_waiting() {
    let (mut b, mut reg, rec) = rig();
    b.dispatch(&mut reg, "telegram:42", "send the email")
        .unwrap();
    rec.lock().unwrap().out.push(approval("a3"));
    b.collect(&reg);
    let before = rec.lock().unwrap().written.len();
    assert_eq!(b.dispatch(&mut reg, "telegram:42", "maybe"), Ok(UNCLEAR));
    assert_eq!(
        rec.lock().unwrap().written.len(),
        before,
        "nothing was sent to the broker on an unclear answer"
    );
    // Still waiting: a proper answer now lands on the same approval.
    assert_eq!(b.dispatch(&mut reg, "telegram:42", "yes"), Ok(ALLOWED));
    assert!(rec.lock().unwrap().written.last().unwrap().contains("a3"));
}

/// Once answered, the conversation goes back to taking tasks.
#[test]
fn after_an_answer_the_next_message_is_a_task_again() {
    let (mut b, mut reg, rec) = rig();
    b.dispatch(&mut reg, "telegram:42", "first").unwrap();
    rec.lock().unwrap().out.push(approval("a4"));
    b.collect(&reg);
    b.dispatch(&mut reg, "telegram:42", "yes").unwrap();
    assert_eq!(
        b.dispatch(&mut reg, "telegram:42", "now do the other thing"),
        Ok(ACK)
    );
    let g = rec.lock().unwrap();
    assert!(g.written.last().unwrap().contains("now do the other thing"));
}

/// An approval belongs to the conversation it came from. Someone else saying "yes" must not
/// approve a stranger's irreversible command.
#[test]
fn a_yes_from_another_address_does_not_answer_this_approval() {
    let (mut b, mut reg, rec) = rig();
    b.dispatch(&mut reg, "telegram:42", "send it").unwrap();
    rec.lock().unwrap().out.push(approval("a5"));
    b.collect(&reg);
    let before = rec.lock().unwrap().written.len();
    // A different address: this is a new conversation, so "yes" is just a task there.
    assert_eq!(b.dispatch(&mut reg, "voice:kitchen", "yes"), Ok(ACK));
    let g = rec.lock().unwrap();
    let sent = g.written.last().unwrap();
    assert!(
        !sent.contains("approve"),
        "another address cannot answer this approval: {sent}"
    );
    assert!(g.written.len() > before);
}
