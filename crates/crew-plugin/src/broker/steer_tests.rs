use std::sync::mpsc;

use super::*;

/// The inbox is one per process, so these tests take turns with it — and
/// start each turn with it empty.
static TURN: Mutex<()> = Mutex::new(());

pub(super) fn turn() -> MutexGuard<'static, ()> {
    let g = TURN.lock().unwrap_or_else(|e| e.into_inner());
    clear();
    g
}

pub(super) type Log = Arc<Mutex<Vec<String>>>;

/// This thread as a task's worker, its announcements written to the log.
pub(super) fn taking(log: &Log) -> Taking {
    let log = Arc::clone(log);
    Taking::begin(Arc::new(move |ev| {
        if let PluginEvent::Steered { channel, text } = ev {
            log.lock()
                .unwrap()
                .push(format!("steered {channel}: {text}"));
        }
    }))
}

pub(super) fn offer(text: &str) {
    deliver("crew".into(), text.into());
}

#[test]
fn a_steer_reaches_the_running_task_once_and_is_announced() {
    let _t = turn();
    let log = Log::default();
    let _task = taking(&log);
    offer("also check the tests");
    assert_eq!(take(), ["also check the tests"]);
    assert!(take().is_empty(), "taken is gone");
    assert_eq!(*log.lock().unwrap(), ["steered crew: also check the tests"]);
}

#[test]
fn a_steer_with_nothing_running_is_ignored() {
    let _t = turn();
    offer("nobody is listening");
    // A task starting now must not find it: the host sends it on its own.
    let _task = taking(&Log::default());
    assert!(take().is_empty());
}

#[test]
fn only_the_task_thread_takes() {
    let _t = turn();
    let _task = taking(&Log::default());
    offer("for the task");
    let elsewhere = std::thread::spawn(take).join().unwrap();
    assert!(elsewhere.is_empty(), "a thread holding no Taking took it");
    assert_eq!(take(), ["for the task"], "and it is still there");
}

#[test]
fn the_last_task_out_empties_the_inbox_and_no_sooner() {
    let _t = turn();
    // Another task, running the whole time until told to end.
    let (ready, go) = (mpsc::channel(), mpsc::channel::<()>());
    let other = std::thread::spawn(move || {
        let _task = taking(&Log::default());
        ready.0.send(()).unwrap();
        go.1.recv().unwrap();
    });
    ready.1.recv().unwrap();

    let first = taking(&Log::default());
    offer("still wanted");
    drop(first);
    let second = taking(&Log::default());
    assert_eq!(take(), ["still wanted"], "another task was still running");
    drop(second);

    offer("left behind");
    go.0.send(()).unwrap();
    other.join().unwrap();
    let _next = taking(&Log::default());
    assert!(take().is_empty(), "the last task out threw it away");
}

#[test]
fn stop_clears_what_is_waiting() {
    let _t = turn();
    let _task = taking(&Log::default());
    offer("too late");
    clear();
    assert!(take().is_empty());
}

#[test]
fn the_section_lists_each_steer_after_its_heading() {
    assert_eq!(section(&[]), "");
    let s = section(&["also check the tests".into(), "no,\nthe other file".into()]);
    assert_eq!(
        s,
        format!("{HEAD}\n- also check the tests\n- no,\n  the other file\n{TAIL}\n\n")
    );
}
