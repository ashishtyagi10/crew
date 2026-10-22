//! `/inbox`: open (or focus) the Gmail pane, `/inbox <search>` to open it on
//! a search in Gmail's own syntax, `/inbox signout` to withdraw the grant.
use crate::app::{CrewApp, FALLBACK_SIZE};
use crate::pane::{Pane, PaneContent};
use crate::spawn::PLACEHOLDER_RECT;

impl CrewApp {
    pub(crate) fn inbox_command(&mut self, arg: &str) {
        match arg.trim() {
            "signout" | "sign out" | "logout" => self.inbox_signout(),
            "calendar" => self.inbox_calendar(),
            q => {
                let i = self.spawn_inbox_pane();
                if let Some(PaneContent::Inbox(p)) = self.panes.get_mut(i).map(|p| &mut p.content) {
                    if !q.is_empty() {
                        p.search(q);
                    }
                }
            }
        }
    }

    /// The one inbox pane: focused when open, made when not.
    fn spawn_inbox_pane(&mut self) -> usize {
        if let Some(i) = self
            .panes
            .iter()
            .position(|p| matches!(p.content, PaneContent::Inbox(_)))
        {
            self.focused = i;
            self.input.focused = false;
            return i;
        }
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        self.panes.push(Pane {
            glide: crate::glide::Glide::default(),
            content: PaneContent::Inbox(Box::new(crate::inbox::InboxPane::new())),
            grid,
            rect: PLACEHOLDER_RECT,
            label: None,
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
            born_ms: crate::anim::now_ms(),
            zoom: None,
        });
        self.focus_new_pane();
        self.panes.len() - 1
    }

    /// `/inbox calendar`: sign in again, asking for Google Calendar's events
    /// beside Gmail's (`inbox::ics::cal`), with the console page that turns
    /// the Calendar API on opened first.
    fn inbox_calendar(&mut self) {
        let Some(grant) = crate::inbox::grant::load() else {
            return self.set_status("/inbox: sign in first, then /inbox calendar");
        };
        let i = self.spawn_inbox_pane();
        crate::linkclick::open_external(crate::inbox::ics::cal::ENABLE);
        if let Some(PaneContent::Inbox(p)) = self.panes.get_mut(i).map(|p| &mut p.content) {
            p.calendar_sign_in(grant.client);
        }
    }

    /// Forget the grant here, and ask Google to withdraw it (off the winit
    /// thread; a sign-out never waits on the network). An open pane goes back
    /// to its setup page.
    fn inbox_signout(&mut self) {
        let Some(grant) = crate::inbox::grant::load() else {
            return self.set_status("/inbox: not signed in");
        };
        crate::inbox::grant::clear();
        std::thread::spawn(move || {
            let _ = crate::inbox::signin::revoke(crate::inbox::signin::REVOKE_URL, &grant);
        });
        for p in &mut self.panes {
            if let PaneContent::Inbox(i) = &mut p.content {
                **i = crate::inbox::InboxPane::new();
            }
        }
        self.set_status("/inbox: signed out, and the grant withdrawn at Google");
        self.redraw();
    }
}

impl CrewApp {
    /// A Cmd+click on the inbox pane: the link drawn under it, at its real
    /// address (only `http`, `https` and `mailto` are ever links there).
    pub(crate) fn resolve_inbox_click(&mut self, i: usize, row: u16, col: u16) -> bool {
        let grid = self.panes[i].grid;
        let PaneContent::Inbox(p) = &self.panes[i].content else {
            return false;
        };
        let Some(url) = p.link_at(grid.cols, grid.rows, row, col) else {
            return false;
        };
        self.follow_inbox_link(&url);
        true
    }

    /// A link out of the inbox, wherever it was picked: an attachment's row
    /// saves and opens the file; anything else opens in the browser.
    pub(crate) fn follow_inbox_link(&mut self, url: &str) {
        if !url.starts_with(crate::inbox::save::ATTACHMENT) {
            crate::linkclick::open_external(url);
            return self.set_status(format!("opening {url}"));
        }
        let i = self
            .panes
            .iter()
            .position(|p| matches!(p.content, PaneContent::Inbox(_)));
        if let Some(PaneContent::Inbox(p)) = i.map(|i| &mut self.panes[i].content) {
            p.note = p.save_attachment(url);
        }
    }
}
