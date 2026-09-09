//! Pictures a document names, decoded once and kept.
//!
//! A `![alt](src)` in a README is a path, and a path is I/O — on the winit
//! thread, inside the frame the document is being drawn for. So the frame
//! never reads one: it asks here, gets `None` the first time, and a worker
//! goes and reads it. The next frame has it.
//!
//! Process-wide rather than per-pane, because a document reopened at a
//! different width, or open in a pane and a window at once, is the same
//! picture — and because the frame that wants it has no `&mut` to hang a
//! cache on.
//!
//! Only local files. A `https://` image in a document is a network fetch a
//! terminal should not be making on its own; it stays alt text.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::Mutex;

use crate::viewpane::bitmap::{self, Bitmap};

/// How many decoded pictures are kept. A document is a few; the cap is for a
/// long session that has opened many.
const KEEP: usize = 48;

enum Entry {
    Loading(Receiver<Option<Bitmap>>),
    Ready(Box<Bitmap>),
    /// Missing, unreadable, or not a picture. Kept so the same path is not
    /// asked for again on every frame forever.
    Failed,
}

static CACHE: Mutex<Option<HashMap<PathBuf, Entry>>> = Mutex::new(None);

/// Where a picture stands, without the picture: what a LAYOUT asks, since
/// it decides how many rows to give one and never needs its pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Probe {
    Ready,
    Loading,
    Failed,
}

/// The picture at `path`, or `None` while it is being read (or if it never
/// arrives). Asking is what starts the read.
pub(crate) fn get(path: &Path) -> Option<Bitmap> {
    match probe(path) {
        Probe::Ready => peek(path),
        _ => None,
    }
}

/// The picture at `path` if it has ALREADY landed — no read is started and
/// none is collected. The paint pass reads through this so a picture's
/// arrival is only ever noticed by the layout, which is the pass that gives
/// it its rows; were the paint to notice first, `loading()` would fall quiet
/// before any layout had seen the picture, and no frame would come to draw
/// it (see `chatimage`).
pub(crate) fn peek(path: &Path) -> Option<Bitmap> {
    match lock().as_ref()?.get(path)? {
        Entry::Ready(bm) => Some((**bm).clone()),
        _ => None,
    }
}

/// Where `path` stands. Asking for a path never asked for starts its read.
pub(crate) fn probe(path: &Path) -> Probe {
    let mut g = lock();
    let map = g.get_or_insert_with(HashMap::new);
    match map.get(path) {
        Some(Entry::Ready(_)) => return Probe::Ready,
        Some(Entry::Failed) => return Probe::Failed,
        Some(Entry::Loading(_)) => {}
        None => {
            if map.len() >= KEEP {
                map.clear();
            }
            let (tx, rx) = mpsc::channel();
            let p = path.to_path_buf();
            std::thread::spawn(move || {
                let bm = std::fs::read(&p).ok().and_then(|b| bitmap::decode(&b));
                let _ = tx.send(bm);
            });
            map.insert(path.to_path_buf(), Entry::Loading(rx));
            return Probe::Loading;
        }
    }
    // Loading: see whether the worker has finished, without blocking the
    // frame on it.
    let done = match map.get(path) {
        Some(Entry::Loading(rx)) => match rx.try_recv() {
            Ok(Some(bm)) => Some(Entry::Ready(Box::new(bm))),
            Ok(None) | Err(mpsc::TryRecvError::Disconnected) => Some(Entry::Failed),
            Err(mpsc::TryRecvError::Empty) => None,
        },
        _ => None,
    };
    match done {
        Some(e @ Entry::Ready(_)) => {
            map.insert(path.to_path_buf(), e);
            Probe::Ready
        }
        Some(e) => {
            map.insert(path.to_path_buf(), e);
            Probe::Failed
        }
        None => Probe::Loading,
    }
}

/// Whether a worker is still out — the term that keeps frames coming until
/// every picture in view has landed, and stops as soon as they have.
pub(crate) fn loading() -> bool {
    lock()
        .as_ref()
        .is_some_and(|m| m.values().any(|e| matches!(e, Entry::Loading(_))))
}

/// Resolve a document's `![alt](src)` against the file it was written in.
/// `None` for anything that is not a local path this process can open.
pub(crate) fn resolve(src: &str, doc: &Path) -> Option<PathBuf> {
    locate(src, doc.parent()).filter(|p| p.is_file())
}

/// The local path `src` names, relative paths taken from `dir` — a lexical
/// answer, no stat: whether the file exists is the worker's finding (see
/// [`probe`]), so a chat card laying out a hundred messages asks the disk
/// nothing. `None` for a remote or inline source, and for a relative path
/// with no directory to be relative to.
pub(crate) fn locate(src: &str, dir: Option<&Path>) -> Option<PathBuf> {
    if src.contains("://") || src.starts_with("data:") {
        return None;
    }
    let p = Path::new(src);
    match p.is_absolute() {
        true => Some(p.to_path_buf()),
        false => Some(dir?.join(p)),
    }
}

/// Whether a read is still out for exactly this path — what a test asks,
/// since [`loading`] is a property of the whole process and the suite runs
/// its cases at once.
#[cfg(test)]
pub(crate) fn pending(path: &Path) -> bool {
    lock()
        .as_ref()
        .and_then(|m| m.get(path))
        .is_some_and(|e| matches!(e, Entry::Loading(_)))
}

fn lock() -> std::sync::MutexGuard<'static, Option<HashMap<PathBuf, Entry>>> {
    CACHE.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
#[path = "imgcache_tests.rs"]
mod imgcache_tests;
