//! Window-event dispatch: mouse focus/zoom/paste/scroll, keyboard forwarding,
//! resize, scale changes, and redraw — split out of the `ApplicationHandler`
//! impl so each surface stays small.
use std::time::{Duration, Instant};

use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::ModifiersState;

use crate::app::CrewApp;

/// Max gap between two left clicks on the same pane to count as a double-click.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

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
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
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
                // Focus the surface and arm a drag selection on a terminal pane.
                if let Some(i) = self.selection_press() {
                    // A second click on the same pane within 400ms toggles zoom;
                    // cancel the just-armed drag so the release doesn't copy.
                    let now = Instant::now();
                    let double = self
                        .last_click
                        .is_some_and(|(t, pi)| pi == i && now.duration_since(t) < DOUBLE_CLICK);
                    if double {
                        self.zoomed = !self.zoomed;
                        self.last_click = None;
                        self.drag = None;
                    } else {
                        self.last_click = Some((now, i));
                    }
                }
                self.redraw();
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                // A drag that moved finalizes + copies the selection; a plain
                // click (no movement) was already handled on press.
                self.selection_release();
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
