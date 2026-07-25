//! The recently-picked-models list backing `modelpick::rows`'s leading
//! "recent" section: a process-global published at config load and after
//! every pick, since `rows()` keeps its original `(query, current)`
//! signature — the chat pane that calls it (via `chatpalette::after_edit`)
//! has no config handle to thread one through. Split from `modelpick.rs`
//! (child module) to keep that file under the house line cap.
use std::sync::Mutex;

/// Cap on the recent section — beyond a handful it stops being a shortcut.
pub(crate) const MAX_RECENTS: usize = 5;

static RECENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Publish the persisted recents list. Called once at config load
/// (`handler.rs`) and again after every pick (`poll.rs`'s drain of
/// `ChatPane::pending_recent`). A poisoned lock is left as-is rather than
/// panicking — but note that's not cosmetic: `std::sync::Mutex::lock` on a
/// poisoned lock returns `Err` *permanently* (nothing here ever calls
/// `clear_poison`/`into_inner`), so the recents section would stay empty for
/// the rest of the process's life, not just "one tick behind". In practice
/// this is inert rather than dangerous: neither `set_recents` nor
/// `recents_now` ever panics while holding the lock (an assignment and a
/// clone), so there's nothing here to poison it in the first place.
pub(crate) fn set_recents(list: Vec<String>) {
    if let Ok(mut g) = RECENTS.lock() {
        *g = list;
    }
}

pub(crate) fn recents_now() -> Vec<String> {
    RECENTS.lock().map(|g| g.clone()).unwrap_or_default()
}
