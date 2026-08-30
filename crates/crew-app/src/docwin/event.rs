//! Events for a document window: the same keys the viewer pane answers, plus
//! the three a window has that a tile does not — resize, close, and redraw.
use winit::event::{MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::ModifiersState;
use winit::window::WindowId;

use crate::app::CrewApp;
use crate::viewpane::caret::Step;
use crate::viewpane::ViewAction;
use winit::keyboard::{Key, NamedKey};

/// What a key means to a document being edited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Edit {
    Move(Step),
    Type(String),
    /// The same movement, dragging a selection behind it.
    Select(Step),
    SelectAll,
    Backspace,
    Delete,
    Newline,
    Save,
    Undo,
    Redo,
    /// Wrap (or unwrap) the selection in this marker — `**` or `*`.
    Wrap(&'static str),
    Copy,
    Cut,
    Paste,
}

/// Classify a key press for an editing window. `None` leaves it to the
/// viewer's own keys (Esc, the search, the scroll), which still apply.
pub(crate) fn edit_key(k: &winit::event::KeyEvent, mods: ModifiersState) -> Option<Edit> {
    edit_for(&k.logical_key, k.state.is_pressed(), mods)
}

/// [`edit_key`] over a key the tests can build — winit's `KeyEvent` is
/// `#[non_exhaustive]` and carries a platform field with no `Default`.
pub(crate) fn edit_for(key: &Key, pressed: bool, mods: ModifiersState) -> Option<Edit> {
    if !pressed {
        return None;
    }
    let cmd = mods.super_key() || mods.control_key();
    let moved = |s: Step| match mods.shift_key() {
        true => Some(Edit::Select(s)),
        false => Some(Edit::Move(s)),
    };
    match key {
        Key::Named(NamedKey::ArrowLeft) => moved(Step::Left),
        Key::Named(NamedKey::ArrowRight) => moved(Step::Right),
        Key::Named(NamedKey::ArrowUp) => moved(Step::Up),
        Key::Named(NamedKey::ArrowDown) => moved(Step::Down),
        Key::Named(NamedKey::Home) => moved(Step::Home),
        Key::Named(NamedKey::End) => moved(Step::End),
        Key::Named(NamedKey::Backspace) if !cmd => Some(Edit::Backspace),
        Key::Named(NamedKey::Delete) if !cmd => Some(Edit::Delete),
        Key::Named(NamedKey::Enter) if !cmd => Some(Edit::Newline),
        Key::Named(NamedKey::Space) if !cmd => Some(Edit::Type(" ".into())),
        Key::Named(NamedKey::Tab) if !cmd => Some(Edit::Type("  ".into())),
        Key::Character(s) if cmd => match s.as_str() {
            "s" => Some(Edit::Save),
            "a" => Some(Edit::SelectAll),
            "c" => Some(Edit::Copy),
            "x" => Some(Edit::Cut),
            "v" => Some(Edit::Paste),
            // The markers never appear on screen, so this is the way one gets
            // into the file at all.
            "b" => Some(Edit::Wrap("**")),
            "i" => Some(Edit::Wrap("*")),
            "z" => Some(Edit::Undo),
            // Cmd+Shift+Z arrives as the shifted character, exactly like the
            // `{`/`}` and `T` chords the grid uses.
            "Z" => Some(Edit::Redo),
            _ => None,
        },
        // A letter is a letter. Without the caret this is `r` for reload and
        // `o` for open-externally; with one, the window is an editor and
        // typing `o` has to type an `o`.
        Key::Character(s) => Some(Edit::Type(s.to_string())),
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
        // A document window keeps its own modifier state: it is a separate
        // surface, and the grid's `ModifiersChanged` never reaches it.
        if let WindowEvent::ModifiersChanged(m) = &event {
            if let Some(d) = self.docs.iter_mut().find(|d| d.id() == id) {
                d.mods = m.state();
            }
            return;
        }
        let Some(i) = self.docs.iter().position(|d| d.id() == id) else {
            return;
        };
        let mut close = false;
        let mut save = false;
        let mut copy: Option<String> = None;
        let mut refused = false;
        let mut paste = false;
        let mut edit: Option<std::path::PathBuf> = None;
        let mut external: Option<std::path::PathBuf> = None;
        let mods = self.docs[i].mods;
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
                WindowEvent::CursorMoved { position, .. } => {
                    d.pointer = (position.x as f32, position.y as f32);
                }
                WindowEvent::MouseInput {
                    state: winit::event::ElementState::Pressed,
                    button: winit::event::MouseButton::Left,
                    ..
                } => {
                    // A click means "put the cursor here": the pointer's
                    // pixels, over the frame's one-cell ring, in cells.
                    if let Some((row, col)) = d.cell_at(d.pointer) {
                        d.view.click_caret(row, col, d.grid.cols, d.grid.rows);
                        d.window.request_redraw();
                    }
                }
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
                    // While there is a caret this window is an EDITOR, and
                    // the keys mean what they mean in one: the arrows move
                    // the cursor rather than scrolling, and a letter is a
                    // letter rather than a viewer command.
                    if d.view.caret.is_some() {
                        match edit_key(&k, mods) {
                            Some(Edit::Move(dir)) => {
                                d.view.clear_selection();
                                d.view.move_caret(dir, cols, rows);
                                d.window.request_redraw();
                                return;
                            }
                            Some(Edit::Select(dir)) => {
                                d.view.anchor_here();
                                d.view.move_caret(dir, cols, rows);
                                d.window.request_redraw();
                                return;
                            }
                            Some(Edit::SelectAll) => {
                                d.view.select_all(cols, rows);
                                d.window.request_redraw();
                                return;
                            }
                            Some(Edit::Wrap(marker)) => {
                                refused = !d.view.wrap_selection(marker, cols, rows);
                                d.warned = false;
                                d.window.request_redraw();
                            }
                            Some(Edit::Copy) => {
                                copy = d.view.selected_text();
                            }
                            Some(Edit::Cut) => {
                                copy = d.view.selected_text();
                                d.view.delete_selection(cols, rows);
                                d.warned = false;
                                d.window.request_redraw();
                            }
                            Some(Edit::Paste) => {
                                paste = true;
                                d.warned = false;
                            }
                            // Typing again after a refused close puts the
                            // guard back: the next Esc asks once more rather
                            // than throwing away what was typed since.
                            Some(Edit::Type(text)) => {
                                d.view.insert(&text, cols, rows);
                                d.warned = false;
                                d.window.request_redraw();
                                return;
                            }
                            Some(Edit::Backspace) => {
                                d.view.backspace(cols, rows);
                                d.warned = false;
                                d.window.request_redraw();
                                return;
                            }
                            Some(Edit::Delete) => {
                                d.view.delete(cols, rows);
                                d.warned = false;
                                d.window.request_redraw();
                                return;
                            }
                            Some(Edit::Undo) => {
                                d.view.undo(cols, rows);
                                d.warned = false;
                                d.window.request_redraw();
                                return;
                            }
                            Some(Edit::Redo) => {
                                d.view.redo(cols, rows);
                                d.window.request_redraw();
                                return;
                            }
                            Some(Edit::Newline) => {
                                d.view.newline(cols, rows);
                                d.warned = false;
                                d.window.request_redraw();
                                return;
                            }
                            // NOT an early return: the write happens below,
                            // where the pane is no longer borrowed and the
                            // status line can be set.
                            Some(Edit::Save) => save = true,
                            None => {}
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
        if paste {
            // The clipboard is read HERE rather than in the match above: it
            // is a system call, and the pane is borrowed there.
            let text = crate::clipboard::system_text();
            let d = &mut self.docs[i];
            if let Some(text) = text {
                let (cols, rows) = (d.grid.cols, d.grid.rows);
                d.view.insert(&text, cols, rows);
                d.window.request_redraw();
            }
        }
        if refused {
            self.set_status("select inside one paragraph to make it bold or italic");
        }
        if let Some(text) = copy {
            self.copy_text(text);
        }
        if save {
            let d = &mut self.docs[i];
            let name = d.view.path.display().to_string();
            d.warned = false;
            match d.view.save() {
                Ok(()) => self.set_status(format!("saved {name}")),
                Err(e) => self.set_status(format!("could not save {name}: {e}")),
            }
        }
        if close {
            // Esc on a document with unsaved edits asks once rather than
            // throwing the typing away: the second press closes it.
            let d = &mut self.docs[i];
            match d.view.dirty && !d.warned {
                true => {
                    d.warned = true;
                    self.set_status("unsaved changes — Cmd+S to save, Esc again to discard");
                    self.docs[i].window.request_redraw();
                }
                false => {
                    self.docs.remove(i);
                }
            }
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
