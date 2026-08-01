//! Window-event dispatch: mouse focus/zoom/paste/scroll, keyboard forwarding,
//! resize, scale changes, and redraw — split out of the `ApplicationHandler`
//! impl so each surface stays small.
use std::time::Instant;

use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::ModifiersState;

use crate::app::CrewApp;

/// The click-to-open modifier for this platform: Cmd on macOS (unchanged),
/// Ctrl elsewhere — so Windows/Linux users get the familiar Ctrl+click
/// without touching the mac convention. Drives both the terminal Cmd+click
/// path and the chat markdown-link click path.
fn open_modifier(state: ModifiersState) -> bool {
    if cfg!(target_os = "macos") {
        state.super_key()
    } else {
        state.control_key()
    }
}

impl CrewApp {
    /// Handle one `WindowEvent` for the main window.
    pub(crate) fn handle_window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        // Any deliberate input (key press, click, scroll) stamps the user as
        // active: blocked-pane auto-focus never steals focus inside the
        // [`crate::blocked::USER_IDLE_MS`] window that follows.
        match &event {
            WindowEvent::KeyboardInput { event: k, .. } if k.state.is_pressed() => {
                self.last_input_ms = crate::anim::now_ms();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            }
            | WindowEvent::MouseWheel { .. } => {
                self.last_input_ms = crate::anim::now_ms();
            }
            _ => {}
        }
        match event {
            // The close button gets the SAME confirmation as Cmd+Q. It used to
            // exit outright, which meant the guard protecting running
            // shells/agents from a stray keystroke did nothing about a stray
            // click on the traffic light — the easier of the two to hit by
            // accident, and unlike a keystroke it can't be typed into a pane
            // instead. A second click within the window still closes.
            WindowEvent::CloseRequested => {
                if self.confirm_quit() {
                    event_loop.exit();
                } else {
                    self.redraw();
                }
            }
            WindowEvent::ModifiersChanged(mods) => self.mods = mods,
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as f32, position.y as f32);
                // Extend an in-progress selection as the cursor drags.
                self.selection_drag();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                // Cmd+click (Ctrl+click off mac) opens a URL / file / dir in a
                // terminal pane, or a markdown link in a chat pane.
                if open_modifier(self.mods.state()) && self.cmd_click_at_cursor() {
                    self.redraw();
                    return;
                }
                // The [x] border button closes the pane outright; like [-] it
                // must win over focus/drag so the click does nothing else.
                if let Some(i) = self.close_btn_at_cursor() {
                    self.close_pane(i);
                    self.redraw();
                    return;
                }
                // The [-] border button minimizes the pane into the left nav. It
                // must win over the focus path so the click neither focuses
                // the pane nor arms a drag selection.
                if let Some(i) = self.min_btn_at_cursor() {
                    self.minimize_pane(i);
                    self.redraw();
                    return;
                }
                // A plain click on a foldable system card in a chat pane
                // toggles its collapse — armed here, fired on RELEASE and
                // only if the gesture never became a drag (see `chatfold`):
                // toggling on press shifted the layout under a starting
                // drag-selection. Additive: the click still focuses the pane
                // and arms selection below — but an armed toggle never counts
                // toward a double-click zoom, so folding twice can't
                // accidentally zoom the pane (see `select::click_zoom`).
                let fold_armed = self.fold_press_at_cursor();
                // Focus the surface and arm a drag selection on a terminal pane.
                if let Some(i) = self.selection_press() {
                    self.click_zoom(i, fold_armed);
                }
                self.redraw();
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                // A drag that moved finalizes + copies the selection; a
                // stationary click fires any fold toggle the press armed.
                let dragged = self.selection_release();
                self.fold_release(dragged);
                self.redraw();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                // Right-click pastes into the surface under the cursor.
                self.focus_at_cursor();
                self.paste();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = self.wheel_lines(delta);
                self.scroll_at_cursor(lines);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.on_key_event(event_loop, &event);
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
                // Remember the new logical size to persist (debounced in poll_panes).
                // Skip while maximized so the restore size stays the un-maximized one.
                if let Some(w) = &self.window {
                    if !w.is_maximized() {
                        let scale = w.scale_factor() as f32;
                        self.config.win_w = Some(size.width as f32 / scale);
                        self.config.win_h = Some(size.height as f32 / scale);
                        self.resize_at = Some(Instant::now());
                    }
                }
                self.redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.set_font_size(self.config.font_size * scale_factor as f32);
                }
                self.redraw();
            }
            WindowEvent::RedrawRequested => {
                if self.renderer.is_none() {
                    return;
                }
                // The first frame is the earliest moment a message can
                // actually be seen; `resumed` is not (see `pending_note`).
                if let Some(note) = self.pending_note.take() {
                    self.set_status(note);
                }
                let scenes = self.build_frame();
                // CRT state, refreshed per frame so it tracks live theme changes.
                // Flicker rides the existing busy-anim redraws (poll_panes drives
                // ~15 fps while a pane animates); idle → flicker 0 → static tube.
                let crt_on = self.effective_crt();
                let crt_active = crt_on && self.panes.iter().any(crate::paneview::pane_animating);
                let crt_time = (crate::anim::now_ms() % 100_000) as f32 / 1000.0;
                if let Some(r) = &mut self.renderer {
                    r.set_crt(crt_on);
                    r.set_crt_anim(crt_time, if crt_active { 0.06 } else { 0.0 });
                    r.frame(&scenes);
                }
            }
            // One event per dropped file; routing (and why it targets the
            // FOCUSED pane, not the cursor) lives in `filedrop`.
            WindowEvent::DroppedFile(path) => self.drop_file(&path),
            WindowEvent::ThemeChanged(t) => {
                crew_theme::set_os_dark(t == winit::window::Theme::Dark);
                // An appearance flip lands immediately in auto mode.
                if crew_theme::mode() == Some(crew_theme::RandomMode::Auto) {
                    crew_theme::apply_selection(
                        crew_theme::Selection::Mode(crew_theme::RandomMode::Auto),
                        crate::chattime::unix_now_ms(),
                    );
                    crate::palette::set_accent(self.config.accent_rgb());
                    self.redraw();
                }
            }
            _ => {}
        }
    }
}
