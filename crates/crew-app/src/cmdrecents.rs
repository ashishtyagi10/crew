//! The palette remembers what you actually run.
//!
//! Among commands that match a query equally well, `cmddefs`'s declaration
//! order decided which came first — an order that means something to whoever
//! last edited that file and nothing at all to the person typing. Type `/` on
//! an empty bar and the list opened on whatever happens to be at the top of
//! the table, every time, no matter that you have run `/gradient` forty times
//! and `/clearlog` never.
//!
//! So the palette keeps a most-recently-used list and uses it as the tie-break.
//! The rule that keeps it predictable is that **recency reorders within a
//! match-quality band and never across one**: a prefix match still beats a
//! fuzzy match, always, so typing `/de` can never float something that does
//! not begin with `de` above something that does. A learned list that can
//! reorder the *kind* of match stops being a list you can aim at.
//!
//! Persisted in the config, so it survives a restart — a shortcut that resets
//! every launch is not one. Capped, because past a handful the tail is not
//! recency any more, it is just the whole table in a different arbitrary
//! order.
//!
//! A process-global, published at config load and after each run, for the same
//! reason [`crate::modelrecents`] is one: `suggest::matches` is a free
//! function on a hot input path with no config handle to thread through.
use std::sync::Mutex;

/// How many commands are remembered.
///
/// Ten is about a screen of palette rows: long enough that a working habit is
/// covered, short enough that the list is still visibly *yours* rather than a
/// slow-moving copy of the whole table.
pub(crate) const MAX: usize = 10;

static RECENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Publish the persisted list (most recent first). Called at config load.
pub(crate) fn set(list: Vec<String>) {
    if let Ok(mut g) = RECENTS.lock() {
        *g = list;
        g.truncate(MAX);
    }
}

/// The list as it stands, most recent first.
pub(crate) fn now() -> Vec<String> {
    RECENTS.lock().map(|g| g.clone()).unwrap_or_default()
}

/// Move `name` (with its leading slash) to the front, and return the new list
/// for the caller to persist. A repeat is a move, not a second entry.
pub(crate) fn record(name: &str) -> Vec<String> {
    let mut list = now();
    list.retain(|c| c != name);
    list.insert(0, name.to_string());
    list.truncate(MAX);
    set(list.clone());
    list
}

/// Where `name` sits in the list — lower is more recent. Anything unrun sorts
/// after everything run, which is what makes this a tie-break rather than a
/// filter.
pub(crate) fn rank_of(list: &[String], name: &str) -> usize {
    list.iter().position(|c| c == name).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset() {
        set(Vec::new());
    }

    /// A command run twice is one entry that moved, not two — otherwise the
    /// cap fills with repeats of one habit and the list stops being a summary
    /// of anything.
    #[test]
    fn running_the_same_command_twice_moves_it_rather_than_duplicating() {
        let _g = crate::app::motion_test_guard();
        reset();
        record("/theme");
        record("/gradient");
        let list = record("/theme");
        assert_eq!(list, vec!["/theme", "/gradient"]);
        reset();
    }

    /// The cap holds, and it drops the OLDEST — a cap that dropped the newest
    /// would make the most recent command the one thing never remembered.
    #[test]
    fn the_cap_holds_and_forgets_the_oldest_first() {
        let _g = crate::app::motion_test_guard();
        reset();
        for i in 0..MAX + 5 {
            record(&format!("/c{i}"));
        }
        let list = now();
        assert_eq!(list.len(), MAX);
        assert_eq!(list[0], format!("/c{}", MAX + 4), "newest leads");
        assert!(!list.contains(&"/c0".to_string()), "oldest was forgotten");
        reset();
    }

    /// An unrun command has to sort after every run one, or the tie-break
    /// becomes a filter that hides most of the table.
    #[test]
    fn anything_unrun_sorts_last() {
        let list = vec!["/theme".to_string(), "/font".to_string()];
        assert_eq!(rank_of(&list, "/theme"), 0);
        assert_eq!(rank_of(&list, "/font"), 1);
        assert_eq!(rank_of(&list, "/never-run"), usize::MAX);
    }

    /// A restored list longer than the cap (an old config, a hand-edited one)
    /// must not smuggle a longer history past it.
    #[test]
    fn a_restored_list_is_capped_too() {
        let _g = crate::app::motion_test_guard();
        set((0..MAX + 7).map(|i| format!("/c{i}")).collect());
        assert_eq!(now().len(), MAX);
        reset();
    }
}
