//! `/inbox`: open (or focus) the Gmail pane, `/inbox <search>` to open it on
//! a search in Gmail's own syntax, `/inbox signout` to withdraw the grant.
use crate::app::{CrewApp, FALLBACK_SIZE};
use crate::pane::{Pane, PaneContent};
use crate::spawn::PLACEHOLDER_RECT;

impl CrewApp {
    pub(crate) fn inbox_command(&mut self, arg: &str) {
        match arg.trim() {
            "signout" | "sign out" | "logout" => self.inbox_signout(),
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
        });
        self.focus_new_pane();
        self.panes.len() - 1
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
