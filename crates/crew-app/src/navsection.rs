//! The nav's sections — what the activity bar switches between, the way VS Code's left edge
//! switches between Explorer, Search and the rest.
//!
//! The nav was one dashboard: clock, dials, load, host, net, git, the glance cards or the LOG,
//! and the pane list, stacked down one column and divided by whatever the window's height
//! allowed. Every new thing the nav could show had to fight the others for rows. Sections end
//! that: each is a whole column of its own, picked from the bar — the pane list, the machine,
//! the activity, the file tree, unread mail, and Far's recent folders.
//!
//! **One table.** Everything a section is — its name, the order the bar draws it in (its icon is
//! drawn by [`crate::navicons`]) —
//! is answered here by an exhaustive match, and the bar, its hit path, the layout and `/nav` all
//! read [`NavSection::ALL`]. Adding a section is adding a variant: the compiler lists every
//! place that has to learn it.

/// One section of the nav.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum NavSection {
    /// The tabs and the pane list.
    #[default]
    Panes,
    /// The machine: clock, dials, load, host, net, git.
    System,
    /// What is happening: the glance cards (weather, inbox, serving, waiting) and the LOG.
    Activity,
    /// The working directory as a tree — the slot, for now.
    Explorer,
    /// Unread mail: the newest unread threads, a click opens one (`navinbox`).
    Inbox,
    /// The folders Far panes went to last, a click opens Far there (`navfar`).
    Far,
}

impl NavSection {
    /// Every section, in the order the bar draws them.
    pub(crate) const ALL: [NavSection; 6] = [
        NavSection::Panes,
        NavSection::System,
        NavSection::Activity,
        NavSection::Explorer,
        NavSection::Inbox,
        NavSection::Far,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            NavSection::Panes => "panes",
            NavSection::System => "system",
            NavSection::Activity => "activity",
            NavSection::Explorer => "explorer",
            NavSection::Inbox => "inbox",
            NavSection::Far => "far",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_ascii_lowercase();
        NavSection::ALL.into_iter().find(|n| n.as_str() == s)
    }
}

impl crate::config::CrewConfig {
    /// The section the nav shows; an unknown name falls back to Panes.
    pub(crate) fn nav_section(&self) -> NavSection {
        NavSection::parse(&self.nav_section).unwrap_or_default()
    }
}

#[cfg(test)]
#[path = "navsection_tests.rs"]
mod tests;
