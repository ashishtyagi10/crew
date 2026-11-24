//! The recent list's disk write, on one worker thread. Every viewer open notes a file, and
//! the winit thread must not wait on a disk (a sleeping network home, a full volume) to
//! show one. One worker, not a thread per save: it writes the list as it stands when the
//! write runs, so a burst of notes lands as the newest list, never an older one last.
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;

enum Job {
    Write,
    #[cfg(test)]
    Flush(Sender<()>),
}

static WORKER: Mutex<Option<Sender<Job>>> = Mutex::new(None);

/// The thread the last write ran on — the proof it was not the caller's.
#[cfg(test)]
pub(super) static WROTE_ON: Mutex<Option<std::thread::ThreadId>> = Mutex::new(None);

/// Have the worker write the list. Returns at once.
pub(super) fn request() {
    send(Job::Write);
}

/// Wait for every write asked for so far (tests read the file straight after).
#[cfg(test)]
pub(crate) fn flush() {
    let (tx, rx) = channel();
    send(Job::Flush(tx));
    let _ = rx.recv();
}

fn send(job: Job) {
    let mut g = WORKER.lock().unwrap_or_else(|p| p.into_inner());
    if let Err(std::sync::mpsc::SendError(job)) = g.get_or_insert_with(spawn).send(job) {
        // The worker died (a panic in a write): start another rather than go quiet.
        let tx = spawn();
        let _ = tx.send(job);
        *g = Some(tx);
    }
}

fn spawn() -> Sender<Job> {
    let (tx, rx) = channel::<Job>();
    let _ = std::thread::Builder::new()
        .name("view-recent".into())
        .spawn(move || {
            while let Ok(first) = rx.recv() {
                // Writes queued behind this one are the same write: do it once.
                #[allow(unused_mut)] // only a test's flush is pushed
                let (mut write, mut done) = (false, Vec::<Sender<()>>::new());
                for job in std::iter::once(first).chain(rx.try_iter()) {
                    match job {
                        Job::Write => write = true,
                        #[cfg(test)]
                        Job::Flush(t) => done.push(t),
                    }
                }
                if write {
                    write_now();
                }
                for t in done {
                    let _ = t.send(());
                }
            }
        });
    tx
}

/// Write the list as it is now, one path per line. Not read in yet: nothing to write.
fn write_now() {
    let body: String = {
        let g = super::LIST.lock().unwrap_or_else(|p| p.into_inner());
        let Some(list) = g.as_ref() else {
            return;
        };
        list.iter().map(|d| format!("{}\n", d.display())).collect()
    };
    #[cfg(test)]
    {
        *WROTE_ON.lock().unwrap_or_else(|p| p.into_inner()) = Some(std::thread::current().id());
    }
    let Some(p) = super::path() else {
        return;
    };
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(p, body);
}
