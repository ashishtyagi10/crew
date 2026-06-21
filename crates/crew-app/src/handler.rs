use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::Key;
use winit::window::{Window, WindowId};

use crate::app::{CrewApp, GAP, POLL_MS};
use crate::layout::pane_rects;
use crate::pane::{build_scenes, relayout, spawn_pane, PaneContent};
use crate::session::{key_to_bytes, pane_at};
use crew_render::Renderer;

impl ApplicationHandler for CrewApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes().with_title("Crew");
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));

        match Renderer::new(window.clone()) {
            Ok(renderer) => {
                let initial_grid = Self::current_grid(&renderer);
                self.renderer = Some(renderer);
                self.window = Some(window.clone());

                if let Ok(pane) = spawn_pane("bash", "sh", initial_grid) {
                    self.panes.push(pane);
                    self.focused = 0;
                }
                window.request_redraw();
            }
            Err(e) => {
                eprintln!("GPU init failed: {e:#}");
                event_loop.exit();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }

        // Drain EVERY pane each tick. A `for` loop (not `any()`/`fold`) so all
        // panes are polled for their side effects — `any()` would short-circuit
        // and starve later panes when an earlier one has output.
        let mut any_changed = false;
        let mut collected_actions = Vec::new();
        for p in self.panes.iter_mut() {
            let changed = match &mut p.content {
                PaneContent::Terminal(t) => t.pty.try_read() > 0,
                PaneContent::Chat(c) => {
                    let result = c.poll();
                    collected_actions.extend(result.actions);
                    result.changed
                }
            };
            any_changed |= changed;
        }
        let actions_ran = !collected_actions.is_empty();
        for action in collected_actions {
            use crate::chat::HostAction;
            match action {
                HostAction::SpawnPane {
                    command,
                    args,
                    label,
                } => self.spawn_labeled_terminal(&command, &args, label),
                HostAction::SendPane { label, text } => self.send_to_label(&label, &text),
            }
        }
        if any_changed || actions_ran {
            self.redraw();
        }

        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(POLL_MS),
        ));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
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
                if let Some(renderer) = &self.renderer {
                    let (sw, sh) = renderer.surface_size();
                    let rects = pane_rects(self.panes.len(), sw as f32, sh as f32, GAP);
                    if let Some(i) = pane_at(&rects, self.cursor.0, self.cursor.1) {
                        self.focused = i;
                    }
                }
                self.redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if self.mods.state().super_key() && event.state.is_pressed() {
                    if let Key::Character(s) = &event.logical_key {
                        let s = s.to_string();
                        if self.handle_super_chord(&s) {
                            event_loop.exit();
                        }
                    }
                    self.redraw();
                } else {
                    let focused = self.focused;
                    if let Some(pane) = self.panes.get_mut(focused) {
                        match &mut pane.content {
                            PaneContent::Terminal(t) => {
                                if let Some(bytes) = key_to_bytes(&event) {
                                    if let Err(e) =
                                        t.input.write_all(&bytes).and_then(|_| t.input.flush())
                                    {
                                        eprintln!("pty write error: {e}");
                                    }
                                }
                            }
                            PaneContent::Chat(c) => c.on_key(&event),
                        }
                    }
                    self.redraw();
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
                self.redraw();
            }
            WindowEvent::RedrawRequested => {
                let Some(renderer) = &mut self.renderer else {
                    return;
                };
                if self.panes.is_empty() {
                    return;
                }
                let (cell_w, cell_h) = renderer.cell_size();
                let (sw, sh) = renderer.surface_size();
                let rects = pane_rects(self.panes.len(), sw as f32, sh as f32, GAP);
                if cell_w > 0.0 && cell_h > 0.0 {
                    relayout(&mut self.panes, &rects, cell_w, cell_h);
                }
                let scenes = build_scenes(&self.panes, self.focused);
                renderer.frame(&scenes);
            }
            _ => {}
        }
    }
}

pub fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = CrewApp::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}
