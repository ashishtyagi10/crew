//! The files viewers opened, newest first — what Cmd+P lists before anything is typed
//! (`pathmenu`), and what breaks a tie between two equally good matches once something is
//! (`pathtree::best`). VS Code's and Zed's quick open both lead with the files you were just
//! in: the one you want next is usually one of them.
//!
//! Every door that puts a file in a viewer — `/view`, `/md`, Cmd+P's pick, a clicked path,
//! a session restore, the Explorer (new pane or its reused preview), the document window —
//! goes through [`open`], so none of them can forget to note it. Synthetic files (`/about`,
//! `/out`, `/diff`…) are taken back off when their viewer is marked ephemeral.
//!
//! Persisted as `<config_dir>/crew/view-recent`, one path per line, beside `far-recent`.
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Files kept, newest first.
pub(crate) const MAX: usize = 50;

/// `<config_dir>/crew/view-recent` — or `CREW_VIEW_RECENT_PATH` when set, the seam that keeps
/// tests off the real profile on Windows (as `farrecent::path`).
fn path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CREW_VIEW_RECENT_PATH") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    let p = dirs::config_dir().map(|d| d.join("crew").join("view-recent"))?;
    // Under test only a HOME pointed at a temp dir is written.
    #[cfg(test)]
    if !p.starts_with(std::env::temp_dir()) {
        return None;
    }
    Some(p)
}

/// The list in memory (`None`: not read from disk yet).
static LIST: Mutex<Option<Vec<PathBuf>>> = Mutex::new(None);
/// Bumped on every change, so the picker's memo knows its order went stale.
static GEN: AtomicU64 = AtomicU64::new(0);

/// Open `path` in a viewer, noted as the newest file viewed.
pub(crate) fn open(path: PathBuf) -> crate::viewpane::ViewPane {
    note(&path);
    crate::viewpane::ViewPane::open(path)
}

/// Non-empty lines as paths, duplicates after the first dropped, at most [`MAX`].
pub(crate) fn parse(s: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for l in s.lines().filter(|l| !l.trim().is_empty()) {
        put_back(&mut out, PathBuf::from(l));
    }
    out
}

fn put_back(out: &mut Vec<PathBuf>, p: PathBuf) {
    if !out.contains(&p) && out.len() < MAX {
        out.push(p);
    }
}

/// `file` moved to the front of `list`, the list capped. `true` when it changed.
pub(crate) fn put_front(list: &mut Vec<PathBuf>, file: &Path) -> bool {
    if list.first().is_some_and(|f| f == file) {
        return false;
    }
    list.retain(|p| p != file);
    list.insert(0, file.to_path_buf());
    list.truncate(MAX);
    true
}

/// The recent files, newest first.
pub(crate) fn list() -> Vec<PathBuf> {
    let mut g = LIST.lock().unwrap_or_else(|p| p.into_inner());
    g.get_or_insert_with(load).clone()
}

/// Which version of the list this is — changes whenever a file is noted or forgotten.
pub(crate) fn generation() -> u64 {
    GEN.load(Ordering::Relaxed)
}

/// Note `file` as the newest, by its canonical path (so `/tmp/x` and `/private/tmp/x` are
/// one file, and a picker rooted at either finds it). Saved when the list changed.
pub(crate) fn note(file: &Path) {
    let file = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    edit(|l| put_front(l, &file));
}

/// Take `file` off the list — a synthetic file a viewer showed, not one anybody chose.
pub(crate) fn forget(file: &Path) {
    let file = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    edit(|l| {
        let n = l.len();
        l.retain(|p| p != &file);
        l.len() != n
    });
}

fn edit(f: impl FnOnce(&mut Vec<PathBuf>) -> bool) {
    let mut g = LIST.lock().unwrap_or_else(|p| p.into_inner());
    let list = g.get_or_insert_with(load);
    if f(list) {
        GEN.fetch_add(1, Ordering::Relaxed);
        save(list);
    }
}

/// The recent files that still exist under `root`, newest first, relative to it and
/// `/`-separated — the form the picker's tree rows are in. One stat per listed file, so the
/// picker asks once per [`generation`], not per frame.
pub(crate) fn under(root: &Path) -> Vec<String> {
    let root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    list()
        .iter()
        .filter_map(|p| {
            let rel = p.strip_prefix(&root).ok()?;
            let parts: Vec<_> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect();
            (p.is_file() && !parts.is_empty()).then(|| parts.join("/"))
        })
        .collect()
}

/// [`under`] `root`, remembered until the list changes or the root does — a
/// stat per listed file is cheap once, not once per frame of an open picker.
pub(crate) fn under_cached(root: &Path) -> Rc<Vec<String>> {
    thread_local! {
        static LAST: RefCell<Option<(PathBuf, u64, Rc<Vec<String>>)>> = RefCell::default();
    }
    let gen = generation();
    LAST.with(|l| {
        let mut l = l.borrow_mut();
        match &*l {
            Some((r, g, v)) if r == root && *g == gen => Rc::clone(v),
            _ => {
                let v = Rc::new(under(root));
                *l = Some((root.to_path_buf(), gen, Rc::clone(&v)));
                v
            }
        }
    })
}

fn load() -> Vec<PathBuf> {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| parse(&s))
        .unwrap_or_default()
}

/// Write `list` to the store, one path per line.
fn save(list: &[PathBuf]) {
    let Some(p) = path() else {
        return;
    };
    let body: String = list.iter().map(|d| format!("{}\n", d.display())).collect();
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(p, body);
}

#[cfg(test)]
#[path = "viewrecent_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "viewrecent_doors_tests.rs"]
mod doors_tests;
