//! Docked bottom command bar: a single-line text input drawn as a rounded
//! fieldset card. The working directory rides the top border as the card's
//! legend (`╭─ ~/code/crew ─╮`); the `> text` prompt sits on the interior row.
use std::cell::RefCell;
use std::path::PathBuf;

#[derive(Default)]
pub struct InputBar {
    pub text: String,
    pub focused: bool,
    /// Submitted lines, oldest first — the source for history autosuggestions.
    pub history: Vec<String>,
    /// Highlighted row in the command palette (when it's open).
    pub menu_sel: usize,
    /// Position while browsing history with Up/Down (`None` = editing fresh text).
    pub hist_pos: Option<usize>,
    /// Text typed before history navigation began; Up/Down recall only entries
    /// starting with it (empty = match everything, i.e. plain recall).
    pub hist_prefix: String,
    /// Whether broadcast (synchronized input to all panes) is active.
    pub broadcast: bool,
    /// Crew's working directory: rendered (`~`-abbreviated) as the bar's legend
    /// and used as the base for `cd` directory completion. Empty = none.
    pub cwd: PathBuf,
    /// Memoized `ghost()` result. `ghost()` runs on every render frame, and for
    /// `cd`/`/dump` it does a `read_dir`; without this cache a path
    /// partial sitting in the bar re-scans the directory on every redraw (e.g.
    /// ~15×/s while a pane animates). Interior mutability so the cache fills from
    /// the `&self` render path. Keyed on the inputs `ghost()` actually depends on.
    pub(crate) ghost_cache: RefCell<GhostCache>,
}

/// Cached `ghost()` output and the `(text, menu_sel, cwd)` it was computed for.
#[derive(Default)]
pub(crate) struct GhostCache {
    key: Option<(String, usize, PathBuf)>,
    val: Option<String>,
}

impl InputBar {
    /// The ghost-suffix to show after the typed text (and insert on Tab/→): the
    /// highlighted palette command, else `cd` directory completion, else a
    /// history/slash autosuggestion. `None` when unfocused or nothing completes.
    ///
    /// Memoized: `compute_ghost` can hit the filesystem, but this runs every
    /// frame, so a result is reused until the typed text, palette selection, or
    /// working directory changes.
    pub(crate) fn ghost(&self) -> Option<String> {
        let key = (self.text.clone(), self.menu_sel, self.cwd.clone());
        {
            let cache = self.ghost_cache.borrow();
            if cache.key.as_ref() == Some(&key) {
                return cache.val.clone();
            }
        }
        let val = self.compute_ghost();
        *self.ghost_cache.borrow_mut() = GhostCache {
            key: Some(key),
            val: val.clone(),
        };
        val
    }

    /// Uncached `ghost()` computation — see [`InputBar::ghost`].
    fn compute_ghost(&self) -> Option<String> {
        if !self.focused {
            return None;
        }
        let m = crate::suggest::matches(&self.text);
        if !m.is_empty() {
            let name = m[self.menu_sel.min(m.len() - 1)].name;
            // Only the highlighted command extends inline as ghost text; a fuzzy
            // (non-prefix) match shows no suffix but the palette still lists it
            // and Tab/Enter fills the full name.
            return name.strip_prefix(self.text.as_str()).map(str::to_string);
        }
        if !self.cwd.as_os_str().is_empty() {
            if self.text.starts_with("cd ") {
                return crate::suggest::dir_suggest(&self.text, &self.cwd);
            }
            // `/dump` completes file and directory paths.
            if let Some(p) = crate::pathcomplete::path_suggest(&self.text, &self.cwd) {
                return Some(p);
            }
        }
        crate::suggest::suggest(&self.text, &self.history)
    }
}

#[cfg(test)]
#[path = "inputbar_tests.rs"]
mod tests;
