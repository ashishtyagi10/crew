//! winit `ApplicationHandler` wiring: window creation on resume, and thin
//! delegation of the per-tick poll (`poll.rs`) and window events (`events.rs`).
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use crate::app::CrewApp;
use crate::config::CrewConfig;
use crate::inputbar::InputBar;
use crew_render::Renderer;

impl ApplicationHandler for CrewApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Restore the last window size (logical px), defaulting to 1200x800.
        let w = self.config.win_w.unwrap_or(1200.0).max(400.0);
        let h = self.config.win_h.unwrap_or(800.0).max(300.0);
        let attrs = Window::default_attributes()
            .with_title("Crew")
            .with_resizable(true)
            .with_inner_size(LogicalSize::new(w, h));
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
        self.poll_panes(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        self.handle_window_event(event_loop, event);
    }
}

pub fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let config = CrewConfig::load();
    let cwd = crate::cwd::resolved_start(config.last_dir.as_deref());
    let mut app = CrewApp {
        config,
        // Default focus is the input bar (startup has no panes selected).
        input: InputBar {
            text: String::new(),
            focused: true,
            history: crate::history::load(),
            cwd: cwd.clone(),
            ..Default::default()
        },
        cwd,
        ..Default::default()
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
