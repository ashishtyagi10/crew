use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::Key;
use winit::window::{Window, WindowId};

use crate::app::{CrewApp, POLL_MS};
use crate::config::CrewConfig;
use crate::pane::PaneContent;
use crate::session::key_to_bytes;
use crew_render::Renderer;

impl ApplicationHandler for CrewApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title("Crew")
            .with_resizable(true)
            .with_inner_size(LogicalSize::new(1200.0, 800.0));
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));

        // Font size is in logical points; multiply by the display scale so text is
        // the right physical size on HiDPI/Retina (the surface is in physical px).
        let font_px = self.config.font_size * window.scale_factor() as f32;
        match Renderer::new(window.clone(), font_px) {
            Ok(renderer) => {
                self.renderer = Some(renderer);
                self.window = Some(window.clone());
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
                PaneContent::Settings(_) => false,
            };
            any_changed |= changed;
        }
        if self.sidebar.refresh() {
            any_changed = true;
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
                if let Some(i) = self.pane_at_cursor() {
                    self.focused = i;
                }
                self.redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let mstate = self.mods.state();
                // Cmd+Q / Ctrl+Q quits the app.
                if event.state.is_pressed()
                    && (mstate.super_key() || mstate.control_key())
                    && matches!(&event.logical_key, Key::Character(s) if s.as_str() == "q")
                {
                    event_loop.exit();
                    return;
                }
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
                    let mut applied: Option<CrewConfig> = None;
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
                            PaneContent::Settings(s) => {
                                applied = s.on_key(&event).map(|c| c.config);
                            }
                        }
                    }
                    if let Some(cfg) = applied {
                        self.apply_settings(cfg);
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

pub fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = CrewApp {
        config: CrewConfig::load(),
        ..Default::default()
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
