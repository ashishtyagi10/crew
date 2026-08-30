//! Events for a document window: the same keys the viewer pane answers, plus
//! the three a window has that a tile does not — resize, close, and redraw.
use winit::event::{MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::app::CrewApp;
use crate::viewpane::caret::Step;
use crate::viewpane::ViewAction;

/// The caret movement a key asks for, if it asks for one.
fn caret_step(k: &winit::event::KeyEvent) -> Option<Step> {
    use winit::keyboard::{Key, NamedKey};
    if !k.state.is_pressed() {
        return None;
    }
    match k.logical_key {
        Key::Named(NamedKey::ArrowLeft) => Some(Step::Left),
        Key::Named(NamedKey::ArrowRight) => Some(Step::Right),
        Key::Named(NamedKey::ArrowUp) => Some(Step::Up),
        Key::Named(NamedKey::ArrowDown) => Some(Step::Down),
        Key::Named(NamedKey::Home) => Some(Step::Home),
        Key::Named(NamedKey::End) => Some(Step::End),
        _ => None,
    }
}

impl CrewApp {
    /// Open `path` in a window of its own. Returns whether one appeared.
    pub(crate) fn open_doc_window(&mut self, event_loop: &ActiveEventLoop, path: &str) -> bool {
        let resolved = crate::pathexpand::expand_path(&self.cwd, path);
        if !resolved.is_file() {
            self.set_status(format!("view: not a file: {path}"));
            return false;
        }
        // Already open in a window: raise that one rather than stacking a
        // second copy of the same document on top of it.
        if let Some(d) = self.docs.iter().find(|d| d.view.path == resolved) {
            d.window.focus_window();
            return true;
        }
        let size = self.config.font_size;
        match super::DocWindow::open(event_loop, resolved, size) {
            Some(d) => {
                d.window.request_redraw();
                self.docs.push(d);
                true
            }
            None => {
                self.set_status("could not open a window for it");
                false
            }
        }
    }

    /// Ask for `path` in a window of its own, on the next tick. The file is
    /// checked HERE so a typo answers in the status bar where it was typed,
    /// rather than silently doing nothing a tick later.
    pub(crate) fn queue_doc_window(&mut self, path: &str) {
        let resolved = crate::pathexpand::expand_path(&self.cwd, path);
        match resolved.is_file() {
            true => self.pending_docs.push(resolved),
            false => self.set_status(format!("doc: not a file: {path}")),
        }
    }

    /// Whether `id` belongs to a document window (and so the main window's
    /// event path must not see this event at all).
    pub(crate) fn is_doc_window(&self, id: WindowId) -> bool {
        self.docs.iter().any(|d| d.id() == id)
    }

    /// Handle one event for the document window `id`.
    pub(crate) fn doc_window_event(&mut self, id: WindowId, event: WindowEvent) {
        let Some(i) = self.docs.iter().position(|d| d.id() == id) else {
            return;
        };
        let mut close = false;
        let mut edit: Option<std::path::PathBuf> = None;
        let mut external: Option<std::path::PathBuf> = None;
        {
            let d = &mut self.docs[i];
            match event {
                WindowEvent::CloseRequested => close = true,
                WindowEvent::Resized(size) => {
                    d.renderer.resize(size.width, size.height);
                    d.refit();
                    d.window.request_redraw();
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    let (w, h) = (d.window.inner_size().width, d.window.inner_size().height);
                    d.renderer.resize(w, h);
                    d.refit();
                    d.window.request_redraw();
                }
                WindowEvent::RedrawRequested => d.draw(),
                WindowEvent::MouseWheel { delta, .. } => {
                    // A document scrolls by lines whichever way the mouse
                    // reports; a trackpad's pixel delta is divided by the cell
                    // height, exactly as the pane path does it.
                    let lines = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(p) => {
                            p.y as f32 / d.renderer.cell_size().1.max(1.0)
                        }
                    };
                    d.view.scroll_wheel(d.grid.cols, d.grid.rows, lines as i32);
                    d.window.request_redraw();
                }
                WindowEvent::KeyboardInput { event: k, .. } => {
                    let (cols, rows) = (d.grid.cols, d.grid.rows);
                    // While there is a caret, the arrows move IT. In a viewer
                    // pane they scroll, which is right for a window onto a
                    // file and wrong for a document you are editing.
                    if let Some(dir) = caret_step(&k) {
                        if d.view.caret.is_some() {
                            d.view.move_caret(dir, cols, rows);
                            d.window.request_redraw();
                            return;
                        }
                    }
                    match d.view.on_key(&k, cols, rows, false) {
                        Some(ViewAction::Close) => close = true,
                        Some(ViewAction::Edit(p)) => edit = Some(p),
                        Some(ViewAction::OpenExternal(p)) => external = Some(p),
                        // Already in a window; `w` there is a no-op rather
                        // than a second copy of the same document.
                        Some(ViewAction::Reload) | Some(ViewAction::PopOut(_)) | None => {}
                    }
                    d.window.request_redraw();
                }
                _ => {}
            }
        }
        if close {
            self.docs.remove(i);
        }
        // `$EDITOR` and the OS opener both belong to the app, not to the
        // window: an editor spawns a terminal pane in the grid, which is
        // where a shell lives.
        if let Some(p) = edit {
            self.edit_in_pane(&p.to_string_lossy());
        }
        if let Some(p) = external {
            let _ = open::that_detached(&p);
        }
    }

    /// Open the windows asked for since the last tick. Called from
    /// `about_to_wait`, which is the callback that holds the active event
    /// loop — the one thing a window cannot be created without.
    pub(crate) fn open_pending_docs(&mut self, event_loop: &ActiveEventLoop) {
        for path in std::mem::take(&mut self.pending_docs) {
            self.open_doc_window(event_loop, &path.to_string_lossy());
        }
    }

    /// Step every document window's pending load. Returns whether one landed.
    pub(crate) fn poll_doc_windows(&mut self) -> bool {
        let mut any = false;
        for d in self.docs.iter_mut() {
            if d.poll() {
                d.refit();
                d.window.request_redraw();
                any = true;
            }
        }
        any
    }
}
