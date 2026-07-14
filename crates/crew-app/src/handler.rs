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
        // Taskbar/window icon + app_id so Windows/Linux match the menu entry
        // (macOS gets its icon from the bundle + dockicon::set()).
        #[cfg(not(target_os = "macos"))]
        let attrs = attrs.with_window_icon(crate::appregister::window_icon());
        #[cfg(target_os = "linux")]
        let attrs = {
            use winit::platform::wayland::WindowAttributesExtWayland;
            use winit::platform::x11::WindowAttributesExtX11;
            let attrs = WindowAttributesExtWayland::with_name(attrs, "crew", "crew");
            WindowAttributesExtX11::with_name(attrs, "crew", "crew")
        };
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));

        // Seed the OS appearance for `/theme auto` (ThemeChanged keeps it live).
        if let Some(t) = window.theme() {
            crew_theme::set_os_dark(t == winit::window::Theme::Dark);
            if crew_theme::mode() == Some(crew_theme::RandomMode::Auto) {
                crew_theme::apply_selection(
                    crew_theme::Selection::Mode(crew_theme::RandomMode::Auto),
                    crate::chattime::unix_now_ms(),
                );
                // The re-apply can flip pools (startup guessed OS-dark before
                // the window existed), so refresh the theme-following accent
                // too — same pairing as the ThemeChanged arm in events.rs.
                crate::palette::set_accent(self.config.accent_rgb());
            }
        }

        // Font size is in logical points; multiply by the display scale so text is
        // the right physical size on HiDPI/Retina (the surface is in physical px).
        let font_px = self.config.font_size * window.scale_factor() as f32;
        match Renderer::new(window.clone(), font_px) {
            Ok(mut renderer) => {
                // Apply the persisted font family up front, not just on Save.
                renderer.set_font_family(self.config.font_family.clone());
                renderer.set_paper_texture(self.config.paper_texture);
                renderer.set_paper_grain(self.config.paper_grain);
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

    /// Fires once when the event loop winds down (any quit path — Cmd+Q,
    /// window close, `/exit`): snapshot the open shells' directories so
    /// `/restore` can reopen them next launch.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.save_session();
    }
}

pub fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    // Keep the app-menu entry (Crew.app / .desktop / Start-menu) fresh after
    // updates. Off-thread: registration does file I/O and must never touch
    // the winit thread. CREW_NO_APP_INSTALL=1 opts out (checked inside).
    std::thread::spawn(crate::appregister::auto_register);
    // Runtime Dock icon: terminal launches / symlink-executable bundles have
    // no icon otherwise. Cheap (single NSImage init, no I/O) — safe to run
    // synchronously on the main thread before the event loop starts.
    #[cfg(target_os = "macos")]
    crate::dockicon::set();
    let config = CrewConfig::load();
    // Apply the theme first; the accent default reads the active theme.
    // A saved rotation mode (random/random-dark/random-light/auto) resumes
    // that mode; `theme_id()` would otherwise silently default it to
    // paper-dark (it only parses fixed theme names).
    match config
        .theme
        .as_deref()
        .and_then(crew_theme::parse_selection)
    {
        Some(sel) => crew_theme::apply_selection(sel, crate::chattime::unix_now_ms()),
        None => crew_theme::apply_selection(
            crew_theme::Selection::Fixed(config.theme_id()),
            crate::chattime::unix_now_ms(),
        ),
    }
    // Seed the themeable accent from config before the first frame.
    crate::palette::set_accent(config.accent_rgb());
    // Seed font rotation state: resume the saved on/off flag, but stamp
    // `last_ms` to now so the first rotation only fires after ROTATE_MS (no
    // swap out from under the user at launch).
    let font_rotate = crate::fontrotate::FontRotate {
        on: config.font_random,
        last_ms: crate::chattime::unix_now_ms(),
        ..Default::default()
    };
    let cwd = crate::cwd::resolved_start(config.last_dir.as_deref());
    let saved = crate::sessionsave::saved_count();
    let restore_hint = (saved > 0).then_some(saved);
    // Bind the inter-pane `ask` IPC socket (best-effort — a bind failure just
    // means `crew ask` reports "no crew running"; it never blocks startup).
    let ipc = match crate::ipc::spawn() {
        Ok(h) => Some(h),
        Err(e) => {
            eprintln!("inter-pane ask socket unavailable: {e}");
            None
        }
    };
    // Cross-host federation relay: binds ONLY if the operator opted in with a
    // CREW_FEDERATE_TOKEN. No token → no port, no reachability.
    crate::relay::maybe_spawn_listener();
    let mut app = CrewApp {
        config,
        font_rotate,
        ipc,
        // Default focus is the input bar (startup has no panes selected).
        input: InputBar {
            text: String::new(),
            focused: true,
            history: crate::history::load(),
            cwd: cwd.clone(),
            ..Default::default()
        },
        cwd,
        restore_hint,
        ..Default::default()
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
