//! Window-event dispatch: mouse focus/zoom/paste/scroll, keyboard forwarding,
//! resize, scale changes, and redraw — split out of the `ApplicationHandler`
//! impl so each surface stays small.
use std::time::{Duration, Instant};

use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;

use crate::app::CrewApp;

/// Max gap between two left clicks on the same pane to count as a double-click.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

impl CrewApp {
    /// Handle one `WindowEvent` for the main window.
    pub(crate) fn handle_window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::ModifiersChanged(mods) => self.mods = mods,
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some(i) = self.focus_at_cursor() {
                    // A second click on the same pane within 400ms toggles zoom.
                    let now = Instant::now();
                    let double = self
                        .last_click
                        .is_some_and(|(t, pi)| pi == i && now.duration_since(t) < DOUBLE_CLICK);
                    if double {
                        self.zoomed = !self.zoomed;
                        self.last_click = None;
                    } else {
                        self.last_click = Some((now, i));
                    }
                }
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
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y.round() as i32,
                    MouseScrollDelta::PixelDelta(p) => (p.y / 24.0).round() as i32,
                };
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
                if let Some(r) = &mut self.renderer {
                    r.frame(&scenes);
                }
            }
            _ => {}
        }
    }
}
