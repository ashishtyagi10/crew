//! Session restore's side of [`crate::tabs`].
use crate::app::CrewApp;

/// Session restore's side of tabs: which tab a saved pane was in, and the
/// tabs a restore opens to put them back in.
impl CrewApp {
    /// Record pane-in-tab `tab` on its snapshot: the tab's place and name.
    pub(crate) fn stamp_tab(&self, sp: &mut crate::sessionsave::SavedPane, tab: u32) {
        sp.tab = self.tabs.pos(tab).unwrap_or(0);
        sp.tab_name = self.tabs.name(tab).map(str::to_string);
    }

    /// A tab for each saved tab number, in order: the first is the tab on
    /// screen, the rest open after the last. `(saved number, tab id)`.
    pub(crate) fn restore_tabs(
        &mut self,
        saved: &[crate::sessionsave::SavedPane],
    ) -> Vec<(usize, u32)> {
        let mut out: Vec<(usize, u32)> = Vec::new();
        for sp in saved {
            if out.iter().any(|t| t.0 == sp.tab) {
                continue;
            }
            let id = if out.is_empty() {
                self.tabs.active
            } else {
                self.tabs.push()
            };
            if let (Some(name), Some(t)) = (&sp.tab_name, self.tabs.get_mut(id)) {
                t.name = Some(name.clone());
            }
            out.push((sp.tab, id));
        }
        out
    }

    /// The pane a restore just opened goes in the tab saved number `n` maps to.
    pub(crate) fn retab_last(&mut self, tabs: &[(usize, u32)], n: usize) {
        let id = tabs.iter().find(|t| t.0 == n).map(|t| t.1);
        if let (Some(id), Some(p)) = (id, self.panes.last_mut()) {
            p.tab = id;
        }
    }

    /// After a restore, the first restored tab is the one on screen.
    pub(crate) fn land_on_tab(&mut self, id: Option<u32>) {
        let Some(id) = id else { return };
        if let Some(i) =
            (0..self.panes.len()).find(|&i| self.panes[i].tab == id && !self.panes[i].hidden)
        {
            self.focused = i;
        }
    }
}
