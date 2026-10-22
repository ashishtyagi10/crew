//! The app's half of the inbox's window (`inbox::mailwin`): opening it when
//! `w` asks, redrawing it when the pane changes, and handing its keys, its
//! wheel and its clicks to the one inbox pane. The window closes with the
//! pane, and on Esc from the list, which would otherwise close the pane.
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::app::CrewApp;
use crate::inbox::keys::InboxAction;
use crate::inbox::mailwin::{look, MailWindow};
use crate::inbox::mouse::Clicked;
use crate::pane::PaneContent;

impl CrewApp {
    fn inbox_at(&self) -> Option<usize> {
        self.panes
            .iter()
            .position(|p| matches!(p.content, PaneContent::Inbox(_)))
    }

    pub(crate) fn is_mail_window(&self, id: WindowId) -> bool {
        self.mail.as_ref().is_some_and(|m| m.id() == id)
    }

    /// One tick: open the window `w` asked for, and redraw it when the pane
    /// would now draw something else.
    pub(crate) fn tick_mail_window(&mut self, event_loop: &ActiveEventLoop) {
        let Some(i) = self.inbox_at() else {
            self.mail = None;
            return;
        };
        let PaneContent::Inbox(p) = &mut self.panes[i].content else {
            return;
        };
        let asked = p
            .reading
            .as_mut()
            .is_some_and(|r| std::mem::take(&mut r.pop_out));
        if asked {
            match &self.mail {
                Some(m) => m.window.focus_window(),
                None => {
                    let title = p
                        .reading
                        .as_ref()
                        .map_or("inbox".into(), |r| r.thread.subject.clone());
                    self.mail = MailWindow::open(event_loop, self.config.font_size, &title);
                    if self.mail.is_none() {
                        self.set_status("could not open a window for the thread");
                    }
                }
            }
        }
        let PaneContent::Inbox(p) = &self.panes[i].content else {
            return;
        };
        if let Some(m) = self.mail.as_mut() {
            let now = look(p, m.grid);
            if now != m.seen {
                m.seen = now;
                m.window.request_redraw();
            }
        }
    }

    pub(crate) fn mail_window_event(&mut self, event: WindowEvent) {
        let (Some(i), Some(m)) = (self.inbox_at(), self.mail.as_mut()) else {
            self.mail = None;
            return;
        };
        let PaneContent::Inbox(p) = &mut self.panes[i].content else {
            return;
        };
        let grid = m.grid;
        let mut close = false;
        let mut changed = false;
        let mut open: Option<String> = None;
        match event {
            WindowEvent::CloseRequested => close = true,
            WindowEvent::Resized(size) => {
                m.renderer.resize(size.width, size.height);
                m.refit();
                m.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let size = m.window.inner_size();
                m.renderer.resize(size.width, size.height);
                m.refit();
                m.window.request_redraw();
            }
            WindowEvent::RedrawRequested => m.draw(p),
            WindowEvent::ModifiersChanged(mods) => m.mods = mods.state(),
            // The hand over what clicks, the wash over the row under it, as
            // in the tile.
            WindowEvent::CursorMoved { position, .. } => {
                m.pointer = (position.x as f32, position.y as f32);
                let at = m.cell_at();
                let hand = at.is_some_and(|(r, c)| p.clickable_at(grid.cols, grid.rows, r, c));
                m.window.set_cursor(match hand {
                    true => winit::window::CursorIcon::Pointer,
                    false => winit::window::CursorIcon::Default,
                });
                let hover = at.and_then(|(r, c)| p.thread_at(grid.cols, grid.rows, r, c));
                changed |= std::mem::replace(&mut p.hover, hover) != hover;
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if m.mods.super_key() || m.mods.control_key() => {
                open = m
                    .cell_at()
                    .and_then(|(r, c)| p.link_at(grid.cols, grid.rows, r, c));
            }
            // A plain click does here what it does in the tile: a link, a
            // keycap, the counter, a folded mail, the reply box.
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => match m
                .cell_at()
                .map(|(r, c)| p.click(grid.cols, grid.rows, r, c))
            {
                Some(Clicked::Open(url)) => open = Some(url),
                Some(Clicked::Then(InboxAction::Close)) => close = true,
                Some(_) => changed = true,
                None => {}
            },
            // As the grid's wheel: a slow trackpad's fractions add up.
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = crate::inboxwinkeys::wheel_lines(&mut m.scroll, delta);
                p.scroll_wheel(lines, grid.cols, grid.rows);
                changed = true;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                use crate::inboxwinkeys::{chord, Chord};
                let enter = matches!(
                    event.logical_key,
                    winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter)
                );
                // A chord is a command, as in the grid (`keychord`), never the
                // plain letter: Cmd+D here deleted the thread.
                match crate::keychord::cmd_chord(&event, m.mods, p.composing()) {
                    Some(s) => match chord(&s, enter) {
                        Chord::Close => close = true,
                        Chord::Send => p.send_reply(),
                        Chord::Paste => {
                            if let Some(t) = crate::clipboard::system_text() {
                                p.paste(&t);
                            }
                        }
                        Chord::Nothing => {}
                    },
                    None => {
                        let held = m.mods.super_key() || m.mods.control_key();
                        let act = p.on_key(&event, grid.cols, grid.rows, held);
                        close = matches!(act, Some(InboxAction::Close));
                    }
                }
                if let Some(r) = p.reading.as_mut() {
                    r.pop_out = false; // already out
                }
                changed = true;
            }
            _ => {}
        }
        if let Some(url) = open {
            self.follow_inbox_link(&url);
        }
        if close {
            self.mail = None;
        }
        // The tile shows the same pane: what changed here changed there.
        if changed {
            self.redraw();
        }
    }
}
