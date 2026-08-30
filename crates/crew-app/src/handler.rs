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
        // Always transparency-CAPABLE, even when fully opaque. Transparency is
        // a window-creation attribute — asking for it later would mean tearing
        // the window down — so requesting it up front is what lets
        // the Opacity % setting take effect live instead of after a restart. At
        // opacity 1.0 (the default) the frame is byte-identical to an opaque
        // one, because nothing crew draws leaves alpha below 1.
        let attrs = Window::default_attributes()
            .with_title("Crew")
            .with_resizable(true)
            .with_transparent(true)
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
        // Being transparency-CAPABLE (above) and being transparent are two
        // different things, and only the second is changeable after creation.
        // Drop straight back to opaque unless the setting actually asks for
        // translucency — otherwise macOS keeps `NSWindow.isOpaque` false and
        // draws the title bar against the desktop, which is not ours to paint.
        window.set_transparent(crate::config::wants_window_transparency(
            self.config.window_opacity,
        ));

        // Two things can want the launch note. A crash outranks a version
        // banner — the user watched the window vanish for no stated reason, and
        // saying so is the whole point of `crashlog` — but when BOTH happened
        // (an update landed and then the run died) neither is dropped, because
        // `last_seen_version` is stamped below either way and a suppressed
        // version note would never come back.
        let crash = crate::crashlog::take_report().map(|s| crate::crashlog::crash_note(&s));
        let version =
            crate::appregister::version_change_note(self.config.last_seen_version.as_deref());
        // Held for the first FRAME, not flashed now: a status expires after
        // three seconds and a cold launch takes far longer than that to draw
        // anything, so flashing here would lose the note on exactly the launch
        // it exists for.
        self.pending_note = match (crash, version) {
            (Some(c), Some(v)) => Some(format!("{c} · {v}")),
            (Some(c), None) => Some(c),
            (None, v) => v,
        };
        // One-shot heal when upgrading across 0.12.6: configs written before
        // theme switches cleared overrides can carry a years-old `crt = false`
        // / `glass = "off"` pin that silently guts the CRT theme. Clearing the
        // pins once restores the theme-intended look; anyone who truly wants
        // them off is one `/crt off` away. Runs before the renderer exists, so
        // the first frame already draws the healed config.
        if self
            .config
            .last_seen_version
            .as_deref()
            .is_some_and(|prev| crate::appregister::version_lt(prev, "0.12.6"))
        {
            self.config.reset_look_overrides();
        }
        // One-shot rebalance when upgrading across 0.19.28. `font_smooth`'s
        // old default was making up part of the gamma-encoded blend's deficit
        // as well as doing its own darkening; `/gamma` corrects that honestly
        // now, and the two at their old values deliver more light than the
        // outline asks for. Only the untouched default moves.
        if self
            .config
            .last_seen_version
            .as_deref()
            .is_some_and(|prev| crate::appregister::version_lt(prev, "0.19.28"))
            && self.config.adopt_rebalanced_smoothing()
        {
            self.config.save();
        }
        // One-shot again across 0.19.62, for the same reason one step further
        // on: with the blend corrected in full, the stem darkening only
        // spreads the same light over 45% more pixels. Only the untouched
        // pair moves.
        if self
            .config
            .last_seen_version
            .as_deref()
            .is_some_and(|prev| crate::appregister::version_lt(prev, "0.19.62"))
            && self.config.adopt_undilated_text()
        {
            self.config.save();
        }
        if self.config.last_seen_version.as_deref() != Some(crate::appregister::VERSION) {
            self.config.last_seen_version = Some(crate::appregister::VERSION.to_string());
            self.config.save();
        }

        // Seed the OS appearance for `/theme auto` (ThemeChanged keeps it live).
        if let Some(t) = window.theme() {
            crew_theme::set_os_dark(t == winit::window::Theme::Dark);
            // Re-read the pinned/scheduled flag beside it: winit reports macOS
            // Appearance: Auto as whichever side it is currently showing, so
            // the theme alone cannot tell the two apart.
            self.config.publish_appearance_sources();
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
                renderer.set_font_weight(Some(self.config.font_weight));
                renderer.set_text_smoothing(Some(self.config.font_smooth));
                renderer.set_text_gamma(Some(self.config.font_gamma));
                renderer.set_paper_texture(self.config.paper_texture);
                renderer.set_paper_grain(self.config.paper_grain);
                renderer.set_glass(self.config.glass_level());
                renderer.set_window_opacity(self.config.window_opacity);
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
        // A document window's file is read on a worker; this is the only
        // thing that lands it. Windows asked for elsewhere are opened here
        // too — this is where the active event loop is.
        self.open_pending_docs(event_loop);
        self.poll_doc_windows();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        // The id was discarded for as long as there was only ever one window.
        // A document window is a second surface with its own renderer and its
        // own keys (see `docwin`), and every event that belongs to one must
        // never reach the grid's handler — a resize routed to the wrong
        // window resizes the wrong surface.
        if self.is_doc_window(id) {
            self.doc_window_event(id, event);
            return;
        }
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
    crate::usageledger::init(config.usage_budget_5h, config.usage_budget_7d);
    // Publish the persisted recents once at startup so the `/model` picker's
    // recent section is populated before the first pick (`poll.rs` republishes
    // it after every subsequent pick).
    crate::modelpick::set_recents(config.model_recents.clone());
    // Same shape one list over: the palette's own most-recently-run commands,
    // published once at load so `suggest::matches` can read them without a
    // config handle (see `cmdrecents`).
    crate::cmdrecents::set(config.command_recents.clone());
    // Apply the theme first; the accent default reads the active theme.
    // `theme_selection` is the shared resolution: a saved rotation mode
    // resumes, a saved palette pins, and NO saved theme follows the OS
    // (`auto`). The OS appearance isn't known until the window exists, so
    // a fresh install opens on auto's dark guess and `handler`'s
    // `window.theme()` seed re-applies moments later — through the same
    // develop-fade every theme switch gets. The per-appearance pairing must
    // land BEFORE the apply, or auto's first pick comes from the wrong pool.
    let (pool_dark, pool_light) = config.auto_pool_selections();
    crew_theme::set_auto_pools(pool_dark, pool_light);
    // The clock half of `auto` needs no window, so it lands before the first
    // apply — a Mac pinned to Dark resolves `auto` off the light-hours window
    // from the very first frame instead of opening dark and correcting later.
    config.publish_appearance_sources();
    crew_theme::apply_selection(config.theme_selection(), crate::chattime::unix_now_ms());
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
    // CREW_FEDERATE_TOKEN. No token → no port, no reachability. Built before
    // the app so the listener thread gets a LOG sender (stderr is invisible
    // in a detached GUI run).
    let applog = crate::applog::AppLog::default();
    crate::relay::maybe_spawn_listener(applog.sender());
    let mut app = CrewApp {
        config,
        font_rotate,
        ipc,
        applog,
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
