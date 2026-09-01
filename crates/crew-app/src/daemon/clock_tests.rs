//! What crew does on its own, and the two things it must never get wrong: running a firing
//! twice, and running one with more authority than a schedule is allowed to have.
use crate::channel::loopback::Loopback;
use crate::daemon::intent::Repeat;
use crate::daemon::intentlog::Watchlist;
use crate::daemon::session::{SessionProc, Spawner};
use crate::daemon::Daemon;
use std::path::Path;

/// A session process that exists and does nothing; the registry's own behaviour is covered in
/// `session_tests`.
struct Idle;
impl SessionProc for Idle {
    fn alive(&mut self) -> bool {
        true
    }
    fn kill(&mut self) {}
    fn send(&mut self, _line: &str) -> bool {
        true
    }
    fn output(&self) -> (Vec<String>, usize) {
        (Vec::new(), 0)
    }
}

/// A spawner that remembers what requester each session was opened for, which is the whole
/// security question for a scheduled run.
struct Recorder(std::sync::Arc<std::sync::Mutex<Vec<String>>>);
impl Spawner for Recorder {
    fn spawn(
        &mut self,
        _cwd: Option<&Path>,
        requester: Option<&str>,
    ) -> std::io::Result<Box<dyn SessionProc>> {
        self.0
            .lock()
            .unwrap()
            .push(requester.unwrap_or("<none>").to_string());
        Ok(Box::new(Idle))
    }
}

pub(super) struct Rig {
    pub d: Daemon,
    pub wire: std::sync::Arc<std::sync::Mutex<crate::channel::loopback::Wire>>,
    pub opened: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    pub watch: Watchlist,
}

/// A daemon with a channel that goes nowhere and a watchlist of this test's own — never the
/// user's, which a firing would otherwise consume for real.
pub(super) fn rig(tag: &str) -> Rig {
    let opened = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut d = Daemon::with_spawner(Box::new(Recorder(std::sync::Arc::clone(&opened))));
    let (c, wire) = Loopback::pair("test");
    d.add_channel(Box::new(c));
    let path = std::env::temp_dir().join(format!("crew-clock-{}-{tag}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&path);
    d.set_watchlist(Watchlist::at(&path));
    Rig {
        d,
        wire,
        opened,
        watch: Watchlist::at(&path),
    }
}

pub(super) fn sent(r: &Rig) -> Vec<(String, String)> {
    r.wire.lock().unwrap().outbox.clone()
}

#[test]
fn an_intent_whose_time_has_come_fires_and_says_so_where_it_answers() {
    let mut r = rig("fires");
    r.watch
        .add("the forecast", "test:me", 1_000, Repeat::Once, 0)
        .unwrap();
    r.d.service_intents(1_000);
    let out = sent(&r);
    assert_eq!(out.len(), 1, "one announcement");
    assert_eq!(out[0].0, "test:me", "on the address it was given");
    assert!(out[0].1.contains("the forecast"), "{}", out[0].1);
    assert!(out[0].1.contains("w1"), "and names itself: {}", out[0].1);
}

#[test]
fn an_intent_whose_time_has_not_come_does_nothing_at_all() {
    let mut r = rig("early");
    r.watch
        .add("later", "test:me", 10_000, Repeat::Once, 0)
        .unwrap();
    r.d.service_intents(9_999);
    assert!(sent(&r).is_empty());
    assert!(r.opened.lock().unwrap().is_empty(), "no session was opened");
}

#[test]
fn a_one_shot_fires_exactly_once_however_many_ticks_follow() {
    // The poll runs four times a second. A firing that is not recorded before the work is
    // dispatched would go out on every one of them.
    let mut r = rig("once");
    r.watch
        .add("wake me", "test:me", 1_000, Repeat::Once, 0)
        .unwrap();
    r.d.service_intents(1_000);
    r.d.service_intents(1_500);
    r.d.service_intents(9_000);
    assert_eq!(sent(&r).len(), 1);
    assert!(r.watch.live().is_empty(), "and it is off the watchlist");
}

#[test]
fn a_repeat_comes_back_at_its_next_time_and_not_before() {
    let mut r = rig("repeat");
    r.watch
        .add("briefing", "test:me", 1_000, Repeat::Every { secs: 60 }, 0)
        .unwrap();
    r.d.service_intents(1_000);
    r.d.service_intents(30_000);
    assert_eq!(sent(&r).len(), 1, "not again mid-period");
    r.d.service_intents(61_000);
    assert_eq!(sent(&r).len(), 2, "and again when the period is up");
}

#[test]
fn a_scheduled_run_is_a_trigger_and_never_a_person() {
    // `Requester::Trigger` is what makes the gate refuse an irreversible tool call with nobody
    // awake to ask. A firing that opened its session as anything else would hand a schedule the
    // authority of a human at the keyboard.
    let mut r = rig("who");
    r.watch
        .add("the forecast", "test:me", 1_000, Repeat::Once, 0)
        .unwrap();
    r.d.service_intents(1_000);
    assert_eq!(r.opened.lock().unwrap().as_slice(), ["trigger:w1"]);
}

#[test]
fn a_firing_never_borrows_the_session_a_person_is_talking_in() {
    // Same address, two conversations: the phone's own session is a Channel requester and the
    // intent's is a Trigger. Reusing the first would promote the second's tier.
    let mut r = rig("separate");
    r.wire.lock().unwrap().inbox.push(crate::channel::Inbound {
        from: "test:me".into(),
        text: "book me a flight".into(),
    });
    r.d.service_channels();
    r.watch
        .add("the forecast", "test:me", 1_000, Repeat::Once, 0)
        .unwrap();
    r.d.service_intents(1_000);
    let opened = r.opened.lock().unwrap().clone();
    assert_eq!(opened, ["channel:test:me", "trigger:w1"]);
}

#[test]
fn a_firing_that_waited_for_the_machine_to_wake_says_it_is_late() {
    let mut r = rig("late");
    r.watch
        .add("briefing", "test:me", 0, Repeat::Once, 0)
        .unwrap();
    r.d.service_intents(4 * 60 * 60 * 1000);
    let out = sent(&r);
    assert!(out[0].1.contains("4h ago"), "{}", out[0].1);
}

#[test]
fn a_repeat_missed_for_days_says_what_it_will_not_run() {
    let mut r = rig("skipped");
    let day = 86_400_000;
    r.watch
        .add("briefing", "test:me", 0, Repeat::Every { secs: 86_400 }, 0)
        .unwrap();
    r.d.service_intents(3 * day);
    let out = sent(&r);
    assert_eq!(out.len(), 1, "one firing, not three");
    assert!(
        out[0].1.contains("3 earlier firing(s) were missed"),
        "{}",
        out[0].1
    );
    assert_eq!(r.watch.live()[0].fire_ms, 4 * day);
}
