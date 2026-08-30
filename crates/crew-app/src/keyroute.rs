//! Routing a non-chord key press to whichever pane holds focus, and the two
//! chords that act on the focused pane's *kind* rather than on the app (see
//! [`crate::keys`], which intercepts everything global before calling here).
use winit::event::KeyEvent;
use winit::keyboard::ModifiersState;

use crate::app::CrewApp;
use crate::pane::PaneContent;
use crate::session::key_to_bytes;
use crate::settingspane::SettingsAction;

impl CrewApp {
    /// Hand `event` to the focused pane and apply whatever action it asked
    /// for. Called once every key press that no global chord claimed.
    pub(crate) fn route_key_to_focused(&mut self, event: &KeyEvent, mstate: ModifiersState) {
        let focused = self.focused;
        let shift = mstate.shift_key();
        let alt = mstate.alt_key();
        let mut settings_action: Option<SettingsAction> = None;
        let mut far_action: Option<crate::farpane::FarAction> = None;
        let mut chat_action: Option<crate::chatkeys::ChatAction> = None;
        let mut view_action: Option<crate::viewpane::ViewAction> = None;
        let mut todo_action: Option<crate::todopane::TodoAction> = None;
        let mut is_terminal = false;
        let mut swarm_close = false;
        let mut disk_action: Option<crate::diskpane::DiskAction> = None;
        if let Some(pane) = self.panes.get_mut(focused) {
            match &mut pane.content {
                // Terminal input is written below (so broadcast can reach all panes).
                PaneContent::Terminal(_) => is_terminal = true,
                PaneContent::Chat(c) => {
                    chat_action = c.on_key(event, shift, mstate.control_key(), &self.cwd)
                }
                PaneContent::Settings(s) => {
                    settings_action = s.on_key(event, shift);
                }
                PaneContent::Far(f) => {
                    far_action = f.on_key(event, alt);
                }
                // The swarm view is display-only; Escape closes it.
                PaneContent::Swarm(_) => {
                    swarm_close =
                        crate::swarmpane::esc_closes(&event.logical_key, event.state.is_pressed());
                }
                PaneContent::View(v) => {
                    view_action =
                        v.on_key(event, pane.grid.cols, pane.grid.rows, mstate.control_key())
                }
                // The usage pane is a picture: nothing in it takes a key.
                PaneContent::Usage(_) => {}
                // The disk map picks tiles and walks into them.
                PaneContent::Disk(d) => disk_action = d.on_key(event),
                // The dashboard is a picture: nothing in it takes a key.
                PaneContent::Dash(_) => {}
                PaneContent::Todo(t) => {
                    todo_action = t.on_key(
                        event,
                        pane.grid.cols,
                        pane.grid.rows,
                        mstate.control_key(),
                        alt,
                    )
                }
            }
        }
        if let Some(action) = disk_action {
            match action {
                crate::diskpane::DiskAction::Close => {
                    self.close_pane(focused);
                }
                crate::diskpane::DiskAction::Redraw => self.redraw(),
            }
        }
        if swarm_close {
            self.close_pane(focused);
        }
        if let Some(action) = far_action {
            self.apply_far_action(action, focused);
        }
        if let Some(action) = chat_action {
            self.apply_chat_action(action, focused);
        }
        if let Some(action) = view_action {
            use crate::viewpane::ViewAction;
            match action {
                ViewAction::Close => {
                    self.close_pane(focused);
                }
                ViewAction::Reload => {
                    if let Some(PaneContent::View(v)) =
                        self.panes.get_mut(focused).map(|p| &mut p.content)
                    {
                        v.reload();
                    }
                }
                ViewAction::OpenExternal(p) => {
                    let _ = open::that_detached(&p);
                    self.set_status(format!("opening {}", p.display()));
                }
                ViewAction::Edit(p) => self.apply_view_edit(focused, &p),
                // A window cannot be created outside a winit callback that
                // holds the ACTIVE event loop, and this is not one — so the
                // request is queued and `about_to_wait` opens it (see
                // `docwin`). The pane closes: the document moved, it was not
                // copied.
                ViewAction::PopOut(p) => {
                    self.pending_docs.push(p);
                    self.close_pane(focused);
                }
            }
        }
        if let Some(crate::todopane::TodoAction::Close) = todo_action {
            self.close_pane(focused);
        }
        if is_terminal {
            if let Some(bytes) = key_to_bytes(event, mstate.control_key(), shift) {
                self.write_to_terminals(&bytes);
            }
        }
        if let Some(action) = settings_action {
            if let SettingsAction::Apply(cfg) = action {
                self.apply_settings(*cfg);
            }
            // Save and Cancel both close the settings pane.
            self.close_pane(focused);
        }
    }

    /// Cmd+S / Alt+S: save-and-close when the focused pane is a settings
    /// form. Returns `false` when it isn't (the chord keeps its old meaning).
    pub(crate) fn save_focused_settings(&mut self) -> bool {
        let focused = self.focused;
        let Some(pane) = self.panes.get_mut(focused) else {
            return false;
        };
        let PaneContent::Settings(s) = &mut pane.content else {
            return false;
        };
        if let SettingsAction::Apply(cfg) = s.save() {
            self.apply_settings(*cfg);
        }
        self.close_pane(focused);
        true
    }

    /// Ctrl+O toggles `compact_view` on the focused pane if — and only if —
    /// it's a chat pane. Returns `true` when it found one and toggled it
    /// (the caller should stop there); `false` otherwise, so the key keeps
    /// flowing to its old destination (e.g. a terminal's raw byte).
    pub(crate) fn toggle_compact_focused(&mut self) -> bool {
        let Some(pane) = self.panes.get_mut(self.focused) else {
            return false;
        };
        let PaneContent::Chat(c) = &mut pane.content else {
            return false;
        };
        c.compact_view = !c.compact_view;
        true
    }
}
