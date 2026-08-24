//! Registry behaviour: ids are unique and never reused, dead sessions stay visible, and closing
//! actually stops the process.
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A process that is alive until killed (or born dead), counting its kills so a test can prove
/// `close` really reached it.
struct FakeProc {
    alive: bool,
    kills: Arc<AtomicUsize>,
    written: Vec<String>,
}

impl SessionProc for FakeProc {
    fn alive(&mut self) -> bool {
        self.alive
    }
    fn kill(&mut self) {
        self.alive = false;
        self.kills.fetch_add(1, Ordering::SeqCst);
    }
    /// A dead process has no pipe to write to — the same shape as a real broken pipe.
    fn send(&mut self, line: &str) -> bool {
        if !self.alive {
            return false;
        }
        self.written.push(line.to_string());
        true
    }
    /// Echoes back whatever was written, so a test can prove send and poll are wired to the
    /// same session.
    fn output(&self) -> (Vec<String>, usize) {
        (self.written.clone(), 0)
    }
}

/// Hands out `FakeProc`s, or fails on demand.
struct FakeSpawner {
    kills: Arc<AtomicUsize>,
    spawns: Arc<AtomicUsize>,
    born_dead: bool,
    fail: bool,
}

impl Spawner for FakeSpawner {
    fn spawn(
        &mut self,
        _cwd: Option<&Path>,
        _requester: Option<&str>,
    ) -> std::io::Result<Box<dyn SessionProc>> {
        if self.fail {
            return Err(std::io::Error::other("no such program"));
        }
        self.spawns.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(FakeProc {
            alive: !self.born_dead,
            kills: self.kills.clone(),
            written: Vec::new(),
        }))
    }
}

fn registry() -> (Registry, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let kills = Arc::new(AtomicUsize::new(0));
    let spawns = Arc::new(AtomicUsize::new(0));
    let r = Registry::new(Box::new(FakeSpawner {
        kills: kills.clone(),
        spawns: spawns.clone(),
        born_dead: false,
        fail: false,
    }));
    (r, kills, spawns)
}

#[test]
fn opening_a_session_spawns_one_process_and_returns_a_fresh_id() {
    let (mut r, _kills, spawns) = registry();
    let a = r.open("crew", None).unwrap();
    let b = r.open("crew", None).unwrap();
    assert_eq!((a.as_str(), b.as_str()), ("s1", "s2"));
    assert_eq!(spawns.load(Ordering::SeqCst), 2);
    assert_eq!(r.len(), 2);
}

/// A closed id must never come back around: a stale client still holding `s1` would otherwise
/// close a session that belongs to somebody else.
#[test]
fn ids_are_not_reused_after_a_close() {
    let (mut r, _k, _s) = registry();
    let first = r.open("crew", None).unwrap();
    r.close(&first).unwrap();
    let second = r.open("crew", None).unwrap();
    assert_ne!(first, second);
    assert_eq!(second, "s2");
}

#[test]
fn closing_kills_the_process_and_reports_it_was_running() {
    let (mut r, kills, _s) = registry();
    let id = r.open("crew", None).unwrap();
    assert_eq!(r.close(&id), Some(true), "it was alive when closed");
    assert_eq!(kills.load(Ordering::SeqCst), 1, "close reached the process");
    assert_eq!(r.len(), 0);
    assert_eq!(r.close(&id), None, "closing twice finds nothing");
}

/// A session whose process died on its own is still listed — reporting only live ones would
/// read as "never opened", which is a different and false statement.
#[test]
fn a_dead_session_stays_listed_and_closes_as_not_alive() {
    let kills = Arc::new(AtomicUsize::new(0));
    let mut r = Registry::new(Box::new(FakeSpawner {
        kills: kills.clone(),
        spawns: Arc::new(AtomicUsize::new(0)),
        born_dead: true,
        fail: false,
    }));
    let id = r.open("crew", None).unwrap();
    let cards = r.cards();
    assert_eq!(cards.len(), 1);
    assert!(!cards[0].alive, "a dead process is listed as not alive");
    assert_eq!(
        r.close(&id),
        Some(false),
        "close reports it was already dead"
    );
}

#[test]
fn a_spawn_failure_registers_nothing() {
    let mut r = Registry::new(Box::new(FakeSpawner {
        kills: Arc::new(AtomicUsize::new(0)),
        spawns: Arc::new(AtomicUsize::new(0)),
        born_dead: false,
        fail: true,
    }));
    assert!(r.open("crew", None).is_err());
    assert_eq!(
        r.len(),
        0,
        "a failed spawn must not leave a phantom session"
    );
}

#[test]
fn cards_carry_the_label_and_cwd_they_were_opened_with() {
    let (mut r, _k, _s) = registry();
    let dir = std::env::temp_dir();
    r.open("smith", Some(&dir)).unwrap();
    let cards = r.cards();
    assert_eq!(cards[0].label, "smith");
    assert_eq!(cards[0].cwd, Some(dir.display().to_string()));
}

/// The real spawner, against a real child. Proves the process plumbing (spawn, try_wait, kill)
/// rather than the fake's bookkeeping.
#[cfg(unix)]
#[test]
fn the_process_spawner_starts_and_stops_a_real_child() {
    let mut r = Registry::new(Box::new(ProcSpawner {
        program: PathBuf::from("/bin/cat"),
        args: vec![],
    }));
    let id = r.open("cat", None).unwrap();
    assert!(r.cards()[0].alive, "a just-spawned child is running");
    assert_eq!(r.close(&id), Some(true));
}

// ---- the output buffer ------------------------------------------------------------------

#[test]
fn the_buffer_hands_back_only_what_is_new() {
    let mut b = Buffer::default();
    for l in ["a", "b", "c"] {
        b.push(l.to_string());
    }
    let (lines, next, dropped) = b.since(0);
    assert_eq!(lines, vec!["a", "b", "c"]);
    assert_eq!((next, dropped), (3, 0));
    let (lines, next, _) = b.since(next);
    assert!(lines.is_empty(), "a caught-up cursor sees nothing new");
    assert_eq!(next, 3, "and the cursor does not move");
    assert_eq!(b.since(1).0, vec!["b", "c"]);
}

/// The daemon outlives every client, so an unbounded buffer is a week-long memory leak.
#[test]
fn the_buffer_is_capped_and_counts_what_it_dropped() {
    let mut b = Buffer::default();
    for i in 0..(BUFFER_LINES + 10) {
        b.push(format!("l{i}"));
    }
    let (lines, next, dropped) = b.since(0);
    assert_eq!(dropped, 10, "the oldest 10 fell off the front");
    assert_eq!(lines.len(), BUFFER_LINES);
    assert_eq!(lines[0], "l10", "the oldest surviving line is l10");
    assert_eq!(next, BUFFER_LINES + 10, "the cursor counts every line ever");
}

/// A client that was away long enough for its cursor to fall off the front must be clamped to
/// the oldest surviving line — not panicked on (the naive `start - dropped` underflows), and not
/// silently rewound to replay old output as if it were new.
#[test]
fn a_cursor_that_fell_off_the_front_is_clamped_forward() {
    let mut b = Buffer::default();
    for i in 0..(BUFFER_LINES + 50) {
        b.push(format!("l{i}"));
    }
    let (lines, next, dropped) = b.since(3);
    assert_eq!(dropped, 50);
    assert_eq!(lines.len(), BUFFER_LINES, "clamped to the oldest held line");
    assert_eq!(lines[0], "l50");
    assert_eq!(next, BUFFER_LINES + 50);
}

/// A cursor from the future (a client that saw more than this buffer holds — a daemon restart
/// with a stale client) must not index past the end.
#[test]
fn a_cursor_past_the_end_reads_as_caught_up() {
    let mut b = Buffer::default();
    b.push("only".to_string());
    let (lines, next, _) = b.since(999);
    assert!(lines.is_empty());
    assert_eq!(next, 1);
}

// ---- send / poll through the registry --------------------------------------------------

#[test]
fn a_line_sent_to_a_session_reaches_that_session() {
    let (mut r, _k, _s) = registry();
    let a = r.open("crew", None).unwrap();
    let b = r.open("crew", None).unwrap();
    assert_eq!(r.send(&a, "hello"), Some(true));
    assert_eq!(r.output(&a, 0).unwrap().0, vec!["hello"]);
    assert!(
        r.output(&b, 0).unwrap().0.is_empty(),
        "the other session heard nothing"
    );
}

#[test]
fn sending_to_an_unknown_session_is_a_miss_not_a_silent_success() {
    let (mut r, _k, _s) = registry();
    assert_eq!(r.send("s99", "hello"), None);
    assert!(r.output("s99", 0).is_none());
}

/// A dead process cannot take input. Reporting `true` would tell a caller its command was
/// delivered when nothing received it.
#[test]
fn sending_to_a_dead_session_reports_undelivered() {
    let kills = Arc::new(AtomicUsize::new(0));
    let mut r = Registry::new(Box::new(FakeSpawner {
        kills,
        spawns: Arc::new(AtomicUsize::new(0)),
        born_dead: true,
        fail: false,
    }));
    let id = r.open("crew", None).unwrap();
    assert_eq!(r.send(&id, "hello"), Some(false));
}

/// The survival property, at the level this slice owns it: output written before a client goes
/// away is still there when a client polls from cursor 0 again. The daemon holds the history;
/// losing the reader does not lose the work.
#[cfg(unix)]
#[test]
fn a_sessions_output_outlives_the_client_that_asked_for_it() {
    let mut r = Registry::new(Box::new(ProcSpawner {
        program: PathBuf::from("/bin/cat"),
        args: vec![],
    }));
    let id = r.open("cat", None).unwrap();
    assert_eq!(r.send(&id, "first"), Some(true));
    assert_eq!(r.send(&id, "second"), Some(true));
    // The reader thread is asynchronous; wait for both lines rather than racing it.
    let mut seen = Vec::new();
    for _ in 0..200 {
        seen = r.output(&id, 0).unwrap().0;
        if seen.len() >= 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(seen, vec!["first", "second"], "cat echoed both lines back");
    // A brand-new client, polling from scratch, gets the whole history.
    let (lines, next, dropped) = r.output(&id, 0).unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!((next, dropped), (2, 0));
    // A caught-up client sees nothing new.
    assert!(r.output(&id, next).unwrap().0.is_empty());
    r.close(&id);
}
