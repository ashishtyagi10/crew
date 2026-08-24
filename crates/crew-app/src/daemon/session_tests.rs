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
}

impl SessionProc for FakeProc {
    fn alive(&mut self) -> bool {
        self.alive
    }
    fn kill(&mut self) {
        self.alive = false;
        self.kills.fetch_add(1, Ordering::SeqCst);
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
    fn spawn(&mut self, _cwd: Option<&Path>) -> std::io::Result<Box<dyn SessionProc>> {
        if self.fail {
            return Err(std::io::Error::other("no such program"));
        }
        self.spawns.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(FakeProc {
            alive: !self.born_dead,
            kills: self.kills.clone(),
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
