use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::app::{CrewApp, POLL_MS};
use crate::config::CrewConfig;
use crate::inputbar::InputBar;
use crate::pane::PaneContent;
use crew_render::Renderer;

/// Max gap between two left clicks on the same pane to count as a double-click.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

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
            Ok(mut renderer) => {
                // Apply the persisted font family up front, not just on Save.
                renderer.set_font_family(self.config.font_family.clone());
                if self.config.maximized {
                    window.set_maximized(true);
                }
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
        let focused = self.focused;
        for (i, p) in self.panes.iter_mut().enumerate() {
            let mut rang = false;
            let changed = match &mut p.content {
                PaneContent::Terminal(t) => {
                    let n = t.pty.try_read() > 0;
                    rang = t.pty.take_bell();
                    n
                }
                PaneContent::Chat(c) => {
                    let result = c.poll();
                    collected_actions.extend(result.actions);
                    result.changed
                }
                PaneContent::Settings(_) => false,
            };
            // Output / bells in a pane you're not watching flag it.
            if i != focused {
                p.activity |= changed;
                p.bell |= rang;
            }
            any_changed |= changed || rang;
        }
        if self.sidebar.refresh() {
            any_changed = true;
        }
        // Animate the matrix-rain welcome screen while there are no panes.
        if self.panes.is_empty() {
            self.tick = self.tick.wrapping_add(1);
            any_changed = true;
        }
        // Close terminal panes whose shell has exited (e.g. the user typed `exit`).
        let exited: Vec<usize> = self
            .panes
            .iter()
            .enumerate()
            .filter(|(_, p)| matches!(&p.content, PaneContent::Terminal(t) if t.pty.exited()))
            .map(|(i, _)| i)
            .collect();
        if !exited.is_empty() {
            for i in exited.into_iter().rev() {
                self.close_pane(i);
            }
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
        // Honour OSC 52 copy requests from terminal programs.
        if let Some(text) = self.take_pane_clipboard() {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_text(text);
            }
        }
        self.sync_window_title();

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
        // Default focus is the input bar (startup has no panes selected).
        input: InputBar {
            text: String::new(),
            focused: true,
            history: crate::history::load(),
            ..Default::default()
        },
        ..Default::default()
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
