//! The folders Far panes went to, newest first — what the nav's Far section lists
//! (`navfar`).
//!
//! A Far panel changes folder in half a dozen places (Enter, `..`, `cd`, a drive pick, a
//! remote coming back local), so nothing is recorded where it happens: once a tick the app
//! looks at each Far pane's ACTIVE panel and notes its folder when it differs from the one
//! last seen there ([`CrewApp::tick_far_recent`]). Remote (rclone) folders are not noted —
//! a click could not open them without the remote.
//!
//! A folder leaves the list when the nav's `×` on its row is clicked, or Delete/Backspace is
//! pressed on it with the list holding the keys ([`forget`]); which listed folders are gone
//! from disk is checked on a throttle, not per frame (`fargone`).
//!
//! Persisted as `<config_dir>/crew/far-recent`, one path per line, beside `far-history`.
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Folders kept, newest first.
pub(crate) const MAX: usize = 20;

/// `<config_dir>/crew/far-recent` — or `CREW_FAR_RECENT_PATH` when set, the seam that keeps
/// tests off the real profile on Windows (see `farpane::cmdhist::path`).
fn path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CREW_FAR_RECENT_PATH") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    let p = dirs::config_dir().map(|d| d.join("crew").join("far-recent"))?;
    // Under test only a HOME pointed at a temp dir is written.
    #[cfg(test)]
    if !p.starts_with(std::env::temp_dir()) {
        return None;
    }
    Some(p)
}

/// The list in memory (`None`: not read from disk yet).
static LIST: Mutex<Option<Vec<PathBuf>>> = Mutex::new(None);

fn load() -> Vec<PathBuf> {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| parse(&s))
        .unwrap_or_default()
}

/// Non-empty lines as paths, duplicates after the first dropped, at most [`MAX`].
pub(crate) fn parse(s: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for l in s.lines().filter(|l| !l.trim().is_empty()) {
        let p = PathBuf::from(l);
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out.truncate(MAX);
    out
}

/// `dir` moved to the front of `list`, the list capped. `true` when it changed.
pub(crate) fn put_front(list: &mut Vec<PathBuf>, dir: &Path) -> bool {
    if list.first().is_some_and(|f| f == dir) {
        return false;
    }
    list.retain(|p| p != dir);
    list.insert(0, dir.to_path_buf());
    list.truncate(MAX);
    true
}

/// `dir` taken out of `list`. `true` when it was there.
pub(crate) fn take_out(list: &mut Vec<PathBuf>, dir: &Path) -> bool {
    let n = list.len();
    list.retain(|p| p != dir);
    list.len() != n
}

/// The recent folders, newest first.
pub(crate) fn list() -> Vec<PathBuf> {
    let mut g = LIST.lock().unwrap_or_else(|p| p.into_inner());
    g.get_or_insert_with(load).clone()
}

/// Empty the list in memory — tests that filled it with temp folders they then delete.
#[cfg(test)]
pub(crate) fn clear() {
    *LIST.lock().unwrap_or_else(|p| p.into_inner()) = Some(Vec::new());
    crate::fargone::reset();
}

/// Note `dir` as the newest; saved when the list changed. `true` when it did.
pub(crate) fn note(dir: &Path) -> bool {
    let mut g = LIST.lock().unwrap_or_else(|p| p.into_inner());
    let list = g.get_or_insert_with(load);
    if !put_front(list, dir) {
        return false;
    }
    save(list);
    true
}

/// Take `dir` off the list for good — the nav's `×`, or Delete on the keyboard's cursor. Saved
/// as `note` saves. `true` when it was listed.
pub(crate) fn forget(dir: &Path) -> bool {
    let mut g = LIST.lock().unwrap_or_else(|p| p.into_inner());
    let list = g.get_or_insert_with(load);
    if !take_out(list, dir) {
        return false;
    }
    save(list);
    true
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

impl crate::app::CrewApp {
    /// Once per tick: note the folder each Far pane's active panel now shows, when it moved
    /// since the last look, and — while the nav shows Far — which listed folders are gone
    /// (`fargone`, throttled and off-thread: this starts a look or lands one, never waits on
    /// a stat). `true` when either changed (the Far section repaints).
    pub(crate) fn tick_far_recent(&mut self) -> bool {
        let mut seen = SEEN.lock().unwrap_or_else(|p| p.into_inner());
        let mut changed = false;
        for p in &self.panes {
            let crate::pane::PaneContent::Far(f) = &p.content else {
                continue;
            };
            let Some(dir) = f.active_local_dir() else {
                continue;
            };
            let last = seen.iter_mut().find(|(born, _)| *born == p.born_ms);
            match last {
                Some((_, d)) if *d == dir => continue,
                Some((_, d)) => *d = dir.clone(),
                None => seen.push((p.born_ms, dir.clone())),
            }
            changed |= note(&dir);
        }
        // Forget panes that closed.
        seen.retain(|(born, _)| self.panes.iter().any(|p| p.born_ms == *born));
        drop(seen);
        let shows = self.nav_section() == crate::navsection::NavSection::Far
            && self.config.show_nav
            && !self.config.nav_collapsed;
        changed | crate::fargone::refresh(crate::anim::now_ms(), shows)
    }
}

/// The folder last seen in each Far pane, by the pane's birth time.
static SEEN: Mutex<Vec<(u64, PathBuf)>> = Mutex::new(Vec::new());

#[cfg(test)]
#[path = "farrecent_tests.rs"]
mod tests;
