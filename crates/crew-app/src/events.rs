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
            WindowEvent::ModifiersChanged(mods) => {
                self.mods = mods;
                // Resting on a bare modifier arms the shortcut hints; letting
                // go, or reaching for a second modifier, disarms them. The
                // dwell itself is checked in `poll`, which is already ticking.
                let was = self.peek_since.is_some();
                self.peek_since =
                    crate::keypeek::held_alone(mods.state()).map(|_| crate::anim::now_ms());
                self.peek_drawn = false;
                // Only repaint on the CLOSING edge — the opening edge has
                // nothing to show yet, and an idle crew must not repaint
                // because a thumb touched Cmd.
                if was && self.peek_since.is_none() {
                    self.redraw();
                }
            }
            // Losing the OS focus stops the ambient drift (and regaining it
            // starts it again) — the redraw it asks for is the one thing in
            // crew that would otherwise run for a window nobody is looking at.
            // The repaint is what restarts the loop, so both edges need one.
            WindowEvent::Focused(on) => {
                self.win_focus = Some(on);
                self.redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as f32, position.y as f32);
                // Extend an in-progress selection as the cursor drags.
                self.selection_drag();
                // …scroll to where a carried gutter points…
                if self.gutter_drag_move() {
                    self.redraw();
                }
                // …resize the sidebar if its edge is in hand…
                if self.nav_edge_drag() {
                    self.redraw();
                }
                // …light the card a carried one would land on…
                if self.card_drag_move() {
                    self.redraw();
                }
                // …and repaint when the pointer crossed onto (or off) a
                // border button, so `[-]`/`[x]` light under the cursor.
                self.hover_moved();
                // Whatever it ended up over, the pointer says what it can do.
                self.pointer_sync();
                self.cursor_in = true;
            }
            WindowEvent::CursorLeft { .. } => {
                // The pointer is over another window now. Anything keyed off
                // hover has to let go — a toast stack held by a coordinate the
                // pointer left behind would never expire (see `toast::hold`).
                self.cursor_in = false;
                self.redraw();
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
                // A toast is an overlay: it is drawn on top of whatever pane
                // it rests on, so a click on it must be answered before the
                // pane underneath ever sees the press.
                if self.toast_click() {
                    self.redraw();
                    return;
                }
                // The sidebar's edge is a handle: taking hold of it must win
                // over everything below, or the press lands in whatever pane
                // sits a few pixels to its right.
                if self.nav_edge_press() {
                    return;
                }
                // A scrolled-back pane's right border is a scroll gutter; it
                // must win over the focus/drag path, or the press lands on
                // the last content column instead.
                if self.gutter_press() {
                    self.redraw();
                    return;
                }
                // The strip's `+N` tile reveals the first pane it stands for.
                if self.overflow_click() {
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
                // A click inside a todo pane acts where it lands (checkbox
                // toggles, ✗ deletes, a row selects, the composer refocuses)
                // and focuses the pane; empty regions fall through to the
                // normal focus/drag path below.
                if self.todo_click_at_cursor() {
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
                // accidentally zoom the pane (see `select::click_gesture`).
                let fold_armed = self.fold_press_at_cursor();
                // Focus the surface and arm a drag selection on a terminal pane.
                if let Some(i) = self.selection_press() {
                    self.click_gesture(i, fold_armed);
                    // A press that armed no text selection landed on the
                    // card's legend row — pick the card up (see `panedrag`).
                    self.card_press(i);
                }
                self.pointer_sync();
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
                // A card dropped on another swaps the two; either kind of
                // drag means the gesture was never a click, so no fold fires.
                let swapped = self.card_drop();
                let resized = self.nav_edge_release() || self.gutter_release();
                self.fold_release(dragged || swapped || resized);
                // Letting go changes what the pointer can do next, and a
                // release moves nothing — so the shape is resolved here too.
                self.pointer_sync();
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
                let lines = self.wheel_lines_boosted(delta);
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
                    renderer.set_leading(self.config.leading().ratio());
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
                let crt = self.effective_crt();
                let busy = self.panes.iter().any(crate::paneview::pane_animating);
                let crt_active = crt.is_some() && busy;
                let crt_time = (crate::anim::now_ms() % 100_000) as f32 / 1000.0;
                let fade = self.theme_fade(crate::anim::now_ms());
                // The modern wash's phase is advanced from this frame's delta
                // at whichever pace applies — the busy one, the far slower
                // ambient one, or held (see `washphase`).
                let drift = crew_theme::theme().modern.map_or(0, |m| m.drift_ms);
                let pace = crate::washphase::pace(drift, busy, self.ambient_drift());
                let wash = self
                    .wash
                    .advance(crate::anim::now_ms(), pace, crate::motion::level());
                // ... and the gradient's own colour rides the same clock: one
                // hue offset, published to the theme layer, worn this frame by
                // the wash, the dot lattice and every card's stroke at once.
                // At `gradient off` the span is zero and this is a no-op store
                // of the number that was already there.
                crew_theme::poleshift::set_shift(
                    self.wash.hue_deg(crate::gradientlvl::level().span_deg()),
                );
                if let Some(r) = &mut self.renderer {
                    // Flicker amplitude is the style's own — each phosphor
                    // jitters with its own nerve, not one global 0.06.
                    let amp = if crt_active {
                        crt.map_or(0.0, |s| s.flicker)
                    } else {
                        0.0
                    };
                    r.set_crt(crt);
                    r.set_crt_anim(crt_time, amp);
                    r.set_wash_phase(wash);
                    let (focus, pull) = self.wash_focus.uniform();
                    r.set_wash_focus(focus, pull);
                    r.set_theme_fade(fade);
                    r.frame(&scenes);
                }
            }
            // One event per dropped file; routing (and why it targets the
            // FOCUSED pane, not the cursor) lives in `filedrop`.
            WindowEvent::DroppedFile(path) => self.drop_file(&path),
            WindowEvent::ThemeChanged(t) => {
                crew_theme::set_os_dark(t == winit::window::Theme::Dark);
                // Flipping System Settings between Light/Dark/Auto all arrive
                // here, so re-read whether the appearance is now scheduled or
                // pinned — turning Auto ON must stop the clock fallback, and
                // turning it OFF must start it.
                self.config.publish_appearance_sources();
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
