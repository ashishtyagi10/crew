//! Background processes for `sys:spawn` / `sys:logs` / `sys:kill`.
//!
//! `sys:run` waits for its command and kills the group at the deadline, which
//! is right for a build and useless for a dev server or a watcher: the agent
//! could never start one, hit it, read its log and stop it. This is the other
//! half: the process keeps running between calls, its output lands in a
//! bounded ring, and the TASK owns it.
//!
//! One [`SysBg`] lives in each task's `Session` snapshot, so a process is
//! visible to (and killable by) only the task that started it. Nothing may
//! outlive crew, so three things end a process: `sys:kill`, the task ending
//! ([`SysBg::reap`], from the worker in `stdio::send` and its crash path, and
//! on drop as a backstop), and the broker exiting ([`reap_all`]).
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use super::sysbgbuf::{Buf, Shared, Waited};
use super::sysrun::{kill_group, signal_group};

/// Live processes per task. A fifth is almost always a leak, not a plan.
pub(crate) const MAX_LIVE: usize = 4;
/// What each process keeps of its output: the last 256 KB.
pub(crate) const RING: usize = 256 * 1024;
/// Exited processes kept for `sys:logs` (the oldest go first).
const KEEP_EXITED: usize = 8;

pub(super) struct Proc {
    pub(super) id: u32,
    pgid: u32,
    pub(super) cmd: String,
    pub(super) out: Arc<Shared>,
}

/// Every pgid started and not yet reaped, process-wide, for [`reap_all`].
static LIVE: Mutex<Vec<u32>> = Mutex::new(Vec::new());

/// TERM every group, a beat for the polite, then KILL: `kill_group` for many.
fn sweep(pgids: &[u32]) {
    pgids.iter().for_each(|&g| signal_group("-TERM", g));
    std::thread::sleep(Duration::from_millis(20));
    pgids.iter().for_each(|&g| signal_group("-KILL", g));
    LIVE.lock()
        .unwrap_or_else(|e| e.into_inner())
        .retain(|g| !pgids.contains(g));
}

/// The broker is going away: nothing started here may outlive it.
pub(crate) fn reap_all() {
    let all: Vec<u32> = LIVE.lock().unwrap_or_else(|e| e.into_inner()).clone();
    sweep(&all);
}

pub(crate) struct SysBg {
    procs: Mutex<Vec<Arc<Proc>>>,
    next: AtomicU32,
    cap: usize,
}

impl Default for SysBg {
    fn default() -> Self {
        Self::with_cap(RING)
    }
}

impl SysBg {
    pub(crate) fn with_cap(cap: usize) -> Self {
        Self {
            procs: Mutex::new(Vec::new()),
            next: AtomicU32::new(1),
            cap,
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Arc<Proc>>> {
        self.procs.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(super) fn get(&self, id: u32) -> Result<Arc<Proc>, String> {
        let ids: Vec<String> = self.lock().iter().map(|p| format!("#{}", p.id)).collect();
        self.lock()
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "no background process #{id} in this task; this task's: {}",
                    ids.join(", ")
                )
            })
    }

    /// Start `cmd` in its own group (so the whole tree can be signalled by
    /// pgid, as `sys:run` does), refused past [`MAX_LIVE`] live ones.
    pub(super) fn spawn(&self, cmd: &str) -> Result<Arc<Proc>, String> {
        let mut procs = self.lock();
        let live: Vec<String> = procs
            .iter()
            .filter(|p| p.out.running())
            .map(|p| format!("#{}", p.id))
            .collect();
        if live.len() >= MAX_LIVE {
            return Err(format!(
                "{MAX_LIVE} background processes are already running ({}); sys:kill one first",
                live.join(", ")
            ));
        }
        let mut child = Command::new("/bin/sh")
            .args(["-c", cmd])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .map_err(|e| format!("spawn /bin/sh: {e}"))?;
        let pgid = child.id();
        LIVE.lock().unwrap_or_else(|e| e.into_inner()).push(pgid);
        let out = Arc::new(Shared {
            buf: Mutex::new(Buf {
                open: 2,
                ..Buf::default()
            }),
            cv: Condvar::new(),
            cap: self.cap,
        });
        out.reader(child.stdout.take().expect("piped"));
        out.reader(child.stderr.take().expect("piped"));
        let me = Arc::clone(&out);
        std::thread::spawn(move || {
            let status = child.wait();
            let mut b = me.buf.lock().unwrap_or_else(|e| e.into_inner());
            // A descendant holding the pipe must not hold the exit back.
            b = me
                .cv
                .wait_timeout_while(b, Duration::from_millis(300), |b| b.open > 0)
                .unwrap_or_else(|e| e.into_inner())
                .0;
            b.exit = Some(match status {
                Ok(s) => match (s.code(), s.signal()) {
                    (Some(c), _) => format!("exited with status {c}"),
                    (None, Some(sig)) => format!("killed by signal {sig}"),
                    _ => "exited".into(),
                },
                Err(e) => format!("lost: {e}"),
            });
            me.cv.notify_all();
        });
        let p = Arc::new(Proc {
            id: self.next.fetch_add(1, Ordering::Relaxed),
            pgid,
            cmd: cmd.into(),
            out,
        });
        // Old finished ones make room; the live ones were counted above.
        while procs.len() >= MAX_LIVE + KEEP_EXITED {
            match procs.iter().position(|p| !p.out.running()) {
                Some(i) => drop(procs.remove(i)),
                None => break,
            }
        }
        procs.push(Arc::clone(&p));
        Ok(p)
    }

    /// TERM the group, KILL it if it ignores that for ~2 s; the exit status.
    pub(super) fn kill(&self, id: u32) -> Result<String, String> {
        let p = self.get(id)?;
        signal_group("-TERM", p.pgid);
        if p.out.wait(None, Duration::from_secs(2)) == Waited::TimedOut {
            kill_group(p.pgid);
            p.out.wait(None, Duration::from_secs(2));
        }
        // `sh` gone does not mean the group is: sweep what it backgrounded.
        sweep(&[p.pgid]);
        Ok(p.out.read().2.unwrap_or_else(|| "killed".into()))
    }

    /// The task is over: kill every group it started, running or orphaned.
    pub(crate) fn reap(&self) {
        let pgids: Vec<u32> = self.lock().drain(..).map(|p| p.pgid).collect();
        sweep(&pgids);
    }
}

impl Drop for SysBg {
    fn drop(&mut self) {
        self.reap();
    }
}

#[cfg(test)]
#[path = "sysbg_tests.rs"]
mod tests;
