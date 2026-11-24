//! Which of Far's recent folders (`farrecent`) are gone from disk — drawn muted and struck in
//! the nav (`navfarrow`) rather than found out only by a click.
//!
//! A stat per row per frame is twenty syscalls a frame for an answer that changes once in a
//! blue moon, so the answer is kept here beside the paths: worked out when the nav turns to
//! Far, then at most once every [`EVERY_MS`] while it stays there.
use std::path::PathBuf;
use std::sync::Mutex;

/// The longest a gone folder goes undrawn as gone while the Far section shows.
pub(crate) const EVERY_MS: u64 = 3_000;

/// When the folders were last looked at (`None`: not since the section last showed), and
/// the ones that were gone then.
static GONE: Mutex<(Option<u64>, Vec<PathBuf>)> = Mutex::new((None, Vec::new()));

/// Look again at which listed folders exist, when the section `shows` and the last look is
/// [`EVERY_MS`] old or from before it showed. `true` when the gone set changed.
pub(crate) fn refresh(now_ms: u64, shows: bool) -> bool {
    let mut g = GONE.lock().unwrap_or_else(|p| p.into_inner());
    if !shows {
        // Hidden: the next showing looks at once.
        g.0 = None;
        return false;
    }
    if g.0.is_some_and(|t| now_ms.saturating_sub(t) < EVERY_MS) {
        return false;
    }
    let gone: Vec<PathBuf> = crate::farrecent::list()
        .into_iter()
        .filter(|d| !d.is_dir())
        .collect();
    g.0 = Some(now_ms);
    let changed = g.1 != gone;
    g.1 = gone;
    changed
}

/// The folders gone at the last look.
pub(crate) fn gone() -> Vec<PathBuf> {
    GONE.lock().unwrap_or_else(|p| p.into_inner()).1.clone()
}

/// Whether `dir` was gone at the last look.
#[cfg(test)]
pub(crate) fn is_gone(dir: &std::path::Path) -> bool {
    GONE.lock()
        .unwrap_or_else(|p| p.into_inner())
        .1
        .iter()
        .any(|d| d == dir)
}

/// Nothing gone, nothing looked at — with `farrecent::clear`.
#[cfg(test)]
pub(crate) fn reset() {
    *GONE.lock().unwrap_or_else(|p| p.into_inner()) = (None, Vec::new());
}
