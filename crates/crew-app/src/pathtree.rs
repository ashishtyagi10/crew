//! Go to file: the `/view` picker's matches from the whole tree, not just the
//! one folder the partial names.
//!
//! [`crate::pathmenu`] lists one directory by prefix, which is right for
//! walking but wrong for the far more common case — you know the file is
//! called `explorerhit` and do not know, or care, which folder holds it.
//! VS Code's, Zed's and Cursor's Cmd+P answer with a fuzzy match over every
//! file in the project; this is that. Ranked in four tiers — the file name's
//! start, the query anywhere in the path, its letters in order inside the
//! file name, then across the path — and inside a tier the tighter match and
//! then the shorter path first, so `exhit` puts `explorerhit.rs` above a
//! path that merely contains an e, an x, an h, an i and a t somewhere.
//!
//! The walk is [`crate::fileindex::scan_max`] (bounded in depth and count,
//! build folders skipped), but it runs on a worker: the picker is built on the
//! winit thread, where a cold walk of a large tree would freeze every pane.
//! The first read of a root starts the walk and offers the folder rows alone;
//! the walk lands within a keystroke or two, and is reused for
//! [`FRESH_MS`] before it is walked again.
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver};

/// How long one walk answers for its root before it is walked again (in the
/// background; the old list keeps answering meanwhile).
const FRESH_MS: u64 = 30_000;
/// Most tree rows one read offers: a list you read, under the folder's own.
pub(crate) const MAX_ROWS: usize = 30;
/// Most files one walk keeps. The @-mention's 2,000 is sized for a walk on
/// the frame thread; crew's own `crates/` alone holds ~3,000.
const MAX_FILES: usize = 20_000;

/// One row: the path and the char indices to mark.
pub(crate) type Row = (String, Vec<usize>);

#[derive(Default)]
struct Index {
    root: PathBuf,
    files: Option<Rc<Vec<String>>>,
    at_ms: u64,
    walking: Option<Receiver<Vec<String>>>,
    /// The last query's rows over the current list: the picker is rebuilt
    /// every frame the bar has focus, and ranking 20,000 paths per frame for
    /// a query that has not changed is the waste this saves.
    memo: Option<(String, Rc<Vec<Row>>)>,
}

thread_local! {
    static INDEX: RefCell<Index> = RefCell::default();
}

/// The files under `root` (relative, `/`-separated) as last walked, or `None`
/// while the first walk of this root is still out. Starts a walk when there
/// is none, or the one there is has gone stale.
pub(crate) fn files(root: &Path, now_ms: u64) -> Option<Rc<Vec<String>>> {
    INDEX.with(|ix| {
        let mut ix = ix.borrow_mut();
        if ix.root != root {
            *ix = Index {
                root: root.to_path_buf(),
                ..Index::default()
            };
        }
        if let Some(files) = ix.walking.as_ref().and_then(|rx| rx.try_recv().ok()) {
            ix.files = Some(Rc::new(files));
            ix.memo = None;
            ix.walking = None;
            ix.at_ms = now_ms;
        }
        let stale = ix.files.is_none() || now_ms.saturating_sub(ix.at_ms) > FRESH_MS;
        if stale && ix.walking.is_none() {
            let (tx, rx) = channel();
            let dir = root.to_path_buf();
            std::thread::spawn(move || {
                let _ = tx.send(crate::fileindex::scan_max(&dir, MAX_FILES));
            });
            ix.walking = Some(rx);
        }
        ix.files.clone()
    })
}

/// [`best`] of `root`'s files for `query`, remembered until the query or
/// the list changes. `None` while the first walk is out.
pub(crate) fn rows(root: &Path, query: &str, now_ms: u64) -> Option<Rc<Vec<Row>>> {
    let files = files(root, now_ms)?;
    INDEX.with(|ix| {
        let mut ix = ix.borrow_mut();
        if let Some((q, rows)) = &ix.memo {
            if q == query {
                return Some(Rc::clone(rows));
            }
        }
        let rows = Rc::new(best(&files, query));
        ix.memo = Some((query.to_string(), Rc::clone(&rows)));
        Some(rows)
    })
}

/// The best [`MAX_ROWS`] of `files` for `query`: tier, then span, then the
/// shorter path, then by name; the last tier only when it is all there is.
pub(crate) fn best(files: &[String], query: &str) -> Vec<Row> {
    let q = query.to_lowercase();
    let mut hits: Vec<((u8, usize), &String, Vec<usize>)> = files
        .iter()
        .filter_map(|f| score(f, &q).map(|(rank, at)| (rank, f, at)))
        .collect();
    hits.sort_by(|a, b| (a.0, a.1.len(), a.1).cmp(&(b.0, b.1.len(), b.1)));
    // Letters smeared across a path are a guess worth showing only when
    // nothing better matched: under a real hit they are a dozen rows of noise.
    if hits.first().is_some_and(|h| h.0 .0 < 3) {
        hits.retain(|h| h.0 .0 < 3);
    }
    hits.truncate(MAX_ROWS);
    hits.into_iter().map(|(_, f, at)| (f.clone(), at)).collect()
}

/// `(tier, span)` of `q` (lowercased) in `path`, with the char indices it
/// matched; `None` when it does not match at all.
fn score(path: &str, q: &str) -> Option<((u8, usize), Vec<usize>)> {
    let low = path.to_lowercase();
    let name_b = low.rfind('/').map_or(0, |i| i + 1);
    let name_c = low[..name_b].chars().count();
    let n = q.chars().count();
    let run = |b: usize| {
        let c = low[..b].chars().count();
        (c..c + n).collect::<Vec<_>>()
    };
    if low[name_b..].starts_with(q) {
        return Some(((0, 0), run(name_b)));
    }
    if let Some(b) = low.find(q) {
        return Some(((1, 0), run(b)));
    }
    let span = |at: &[usize]| at.last().unwrap_or(&0) - at.first().unwrap_or(&0);
    if let Some(at) = subsequence(&low[name_b..], q) {
        let at: Vec<usize> = at.into_iter().map(|i| i + name_c).collect();
        return Some(((2, span(&at)), at));
    }
    subsequence(&low, q).map(|at| ((3, span(&at)), at))
}

/// The char indices of `q`'s letters in `hay`, leftmost each, or `None`.
fn subsequence(hay: &str, q: &str) -> Option<Vec<usize>> {
    let mut want = q.chars().peekable();
    let at: Vec<usize> = hay
        .chars()
        .enumerate()
        .filter(|(_, c)| want.next_if_eq(c).is_some())
        .map(|(i, _)| i)
        .collect();
    want.peek().is_none().then_some(at)
}

#[cfg(test)]
#[path = "pathtree_tests.rs"]
mod tests;
