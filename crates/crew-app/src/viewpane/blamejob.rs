//! The `git blame` read behind the viewer's blame gutter — on a worker
//! thread, for the same reason `/diff`'s read is: `git blame` walks a file's
//! whole history, that can take seconds on a big file in a big repo, and
//! every pane in the grid is frozen for as long as anything blocks the winit
//! thread.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use crew_hive::childproc::no_console_window;

use super::blame::{self, Line};

/// Run `git blame` on `path`, from its own directory so the repo is found
/// however the viewer was opened.
fn read(path: &Path) -> Result<Vec<Line>, String> {
    let dir = path.parent().ok_or("blame: no directory")?;
    // `no_console_window` must sit on the line immediately AFTER
    // `Command::new` — the parity test in `crew-hive::childproc` reads the
    // source for exactly that shape, and a comment between the two is enough
    // to make it report the site as unguarded.
    let mut cmd = Command::new("git");
    no_console_window(&mut cmd);
    cmd.args(["--no-pager", "blame", "--line-porcelain", "--"])
        .arg(path)
        .current_dir(dir);
    let out = cmd.output().map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(match why.is_empty() {
            true => "not a file git knows about".into(),
            false => why,
        });
    }
    let lines = blame::parse(&String::from_utf8_lossy(&out.stdout));
    match lines.is_empty() {
        true => Err("nothing to blame".into()),
        false => Ok(lines),
    }
}

/// The blame a viewer pane is showing, or waiting for. `Off` is the default
/// and what `/blame` toggles back to — the gutter is an answer to a question
/// that was asked, not a permanent column.
#[derive(Default)]
pub(crate) enum Blame {
    #[default]
    Off,
    Loading(Receiver<Result<Vec<Line>, String>>),
    On(Vec<Line>),
}

impl Blame {
    /// Start reading `path`. Replaces whatever was there — asking again is
    /// how a stale blame is refreshed.
    pub(crate) fn start(path: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(read(&path));
        });
        Blame::Loading(rx)
    }

    /// The lines to label with, once they are here.
    pub(crate) fn lines(&self) -> Option<&[Line]> {
        match self {
            Blame::On(l) => Some(l),
            _ => None,
        }
    }

    /// Drain the worker. `Some(Err(why))` on the tick it failed — the caller
    /// says why on the status line, since a gutter that never appears cannot
    /// explain itself. `None` while it is still running or already settled.
    pub(crate) fn poll(&mut self) -> Option<Result<(), String>> {
        let Blame::Loading(rx) = self else {
            return None;
        };
        let done = match rx.try_recv() {
            Ok(done) => done,
            Err(TryRecvError::Empty) => return None,
            // The worker died without sending. Settle rather than wait
            // forever for something that is not coming.
            Err(TryRecvError::Disconnected) => Err("the blame was interrupted".into()),
        };
        Some(match done {
            Ok(lines) => {
                *self = Blame::On(lines);
                Ok(())
            }
            Err(why) => {
                *self = Blame::Off;
                Err(why)
            }
        })
    }
}

#[cfg(test)]
#[path = "blamejob_tests.rs"]
mod tests;
