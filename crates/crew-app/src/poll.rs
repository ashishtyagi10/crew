//! The per-tick background work driven by winit's `about_to_wait`: drain every
//! pane's PTY/plugin output, refresh the sidebar, animate the welcome screen,
//! reap exited shells, run host actions, and honour OSC 52 clipboard requests.
use std::time::{Duration, Instant};

use winit::event_loop::{ActiveEventLoop, ControlFlow};

use crate::app::{CrewApp, POLL_MS};
use crate::pane::PaneContent;

/// Poll ticks per rendered frame of the busy progress sweep: the loop runs at
/// ~62 Hz, so redrawing every 4th tick animates the sweep at ~15 fps.
const BUSY_ANIM_DIV: u64 = 4;

impl CrewApp {
    /// One poll cycle. Schedules the next wake-up before returning.
    pub(crate) fn poll_panes(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }

        // Random theme mode: rotate on its 10-minute clock. Cheap + lock-free.
        // Seeds `any_changed` so a rotation repaints every pane in the new theme.
        // A rotation changes the active theme app-wide, so re-apply the themeable
        // accent (it follows the theme when the user hasn't pinned one) exactly
        // like every other theme-change path — otherwise the accent stays frozen
        // on the previous theme after the first rotation.
        let rotated = crew_theme::tick_random(crate::chattime::unix_now_ms());
        if rotated {
            crate::palette::set_accent(self.config.accent_rgb());
        }
        //
        // Drain EVERY pane each tick. A `for` loop (not `any()`/`fold`) so all
        // panes are polled for their side effects — `any()` would short-circuit
        // and starve later panes when an earlier one has output.
        let mut any_changed = rotated;
        // Set when any pane still has buffered PTY output past this tick's read
        // budget. We then keep the loop hot (ControlFlow::Poll) so a flood drains
        // quickly across ticks instead of trickling one budget per 16 ms — while
        // each tick stays bounded, so input and rendering never stall.
        let mut more_pending = false;
        let mut collected_actions = Vec::new();
        // Status flashes from Far panes whose command finished this tick
        // (surfaced after the loop to avoid fighting the panes borrow).
        let mut far_statuses: Vec<String> = Vec::new();
        // Notification events detected this tick, surfaced after the pane loops so
        // they don't fight the `&mut self.panes` borrow. (kind, pane title, detail).
        let mut notify_events: Vec<(crate::notify::NotifyKind, String, String)> = Vec::new();
        let focused = self.focused;
        for (i, p) in self.panes.iter_mut().enumerate() {
            let mut rang = false;
            let mut new_cwd = None;
            let mut matches: Vec<String> = Vec::new();
            let changed = match &mut p.content {
                PaneContent::Terminal(t) => {
                    let n = t.pty.try_read() > 0;
                    more_pending |= t.pty.has_pending();
                    rang = t.pty.take_bell();
                    new_cwd = t.pty.take_cwd();
                    matches = t.pty.take_matches();
                    n
                }
                PaneContent::Chat(c) => {
                    let result = c.poll();
                    collected_actions.extend(result.actions);
                    result.changed
                }
                PaneContent::Swarm(s) => s.poll(),
                // A Far pane changes when its command-line command finishes
                // (panels reload; the result becomes a status flash).
                PaneContent::Far(f) => match f.poll_cmd() {
                    Some(msg) => {
                        far_statuses.push(msg);
                        true
                    }
                    None => false,
                },
                PaneContent::Settings(_) => false,
                // A static file view; nothing to poll (Task 3 adds `r` reload).
                PaneContent::Markdown(_) => false,
            };
            // Follow `cd` inside the pane: a new OSC 7 cwd report retitles the
            // pane to that folder (a `/name` override still wins in title_text).
            if let Some(cwd) = new_cwd {
                p.dir = Some(cwd);
                any_changed = true;
            }
            // Output / bells in a pane you're not watching flag it. A bell or
            // watched pattern also raises the nav-row attention marker — the
            // "needs you" flash for minimized/background panes.
            if i != focused {
                p.activity |= changed;
                p.bell |= rang;
                if rang {
                    crate::attention::raise(
                        p,
                        crate::notify::NotifyKind::Bell,
                        crate::anim::now_ms(),
                    );
                } else if !matches.is_empty() {
                    crate::attention::raise(
                        p,
                        crate::notify::NotifyKind::Pattern,
                        crate::anim::now_ms(),
                    );
                }
            }
            any_changed |= changed || rang;
            // Bell + output-pattern notifications (the pane title is borrowable
            // now that the `&mut p.content` match above has ended).
            if rang || !matches.is_empty() {
                use crate::notify::NotifyKind;
                let title = p.title_text();
                if rang {
                    notify_events.push((NotifyKind::Bell, title.clone(), String::new()));
                }
                for m in matches {
                    notify_events.push((NotifyKind::Pattern, title.clone(), m));
                }
            }
        }
        // Land a finished `?` ask (AI command suggestion) from its worker.
        any_changed |= self.poll_ask();
        if self.sidebar.refresh(&self.cwd) {
            any_changed = true;
        }
        // Name the foreground command in each terminal pane (claude/codex/…), so
        // it rides the title beside the directory. Throttled to ~1×/s.
        if self.procnames.due() {
            // Foreground PID per pane (None = idle shell or non-terminal).
            let fg: Vec<Option<u32>> = self
                .panes
                .iter()
                .map(|p| match &p.content {
                    PaneContent::Terminal(t) => t.pty.foreground_pid(),
                    _ => None,
                })
                .collect();
            let pids: Vec<u32> = fg.iter().flatten().copied().collect();
            self.procnames.refresh(&pids);
            let min = Duration::from_secs(self.config.notify_min_secs);
            let now = Instant::now();
            for (i, (p, fg)) in self.panes.iter_mut().zip(fg).enumerate() {
                let mut just_finished = None;
                if let PaneContent::Terminal(t) = &mut p.content {
                    let cmd = fg.and_then(|pid| self.procnames.name(pid));
                    if t.cmd != cmd {
                        // A foreground command returning to the idle prompt after
                        // running long enough is a "command finished" event.
                        let outcome = crate::notify::agent_done(
                            t.cmd.as_deref(),
                            cmd.as_deref(),
                            t.cmd_since,
                            min,
                            now,
                        );
                        t.cmd = cmd;
                        t.cmd_since = outcome.since;
                        any_changed = true;
                        just_finished = outcome.finished;
                    }
                }
                if let Some(finished) = just_finished {
                    // A command finishing while you're elsewhere raises the
                    // pane's attention marker alongside the notification.
                    if i != focused {
                        crate::attention::raise(
                            p,
                            crate::notify::NotifyKind::AgentDone,
                            crate::anim::now_ms(),
                        );
                    }
                    notify_events.push((
                        crate::notify::NotifyKind::AgentDone,
                        p.title_text(),
                        finished,
                    ));
                }
            }
        }
        // Surface the bell / pattern / command-finished events gathered above.
        // (Pane-exit events are raised at the reap site below.)
        for (kind, pane, detail) in notify_events.drain(..) {
            self.notify(kind, pane, detail);
        }
        for msg in far_statuses.drain(..) {
            self.set_status(&msg);
        }
        // Drive the background self-update: animate its card and dismiss it when
        // done. The new binary applies on `/restart` — Crew does not restart itself.
        if self.update.is_some() {
            let tick = self.poll_update(Instant::now());
            any_changed |= tick.redraw;
        }
        // Clear a status message once it has aged out, repainting the border.
        if self.expire_status() {
            any_changed = true;
        }
        // Animate the welcome screen while there are no panes — but
        // only redraw every Nth tick, so the idle screen runs at ~20 fps, not 60.
        if self.panes.is_empty() {
            self.tick = self.tick.wrapping_add(1);
            if crate::welcome::anim_should_redraw(self.tick) {
                any_changed = true;
            }
        } else if self.panes.iter().any(crate::paneview::pane_animating)
            || crate::attention::any_pulsing(&self.panes, crate::anim::now_ms())
        {
            // Drive the indeterminate progress sweep while any pane is busy
            // (or a chat card is fading in, or an attention marker is still
            // blinking), throttled to ~15 fps so a working pane stays lively
            // without spinning the CPU. Idle → no redraws: a settled attention
            // marker draws once and then costs nothing.
            self.tick = self.tick.wrapping_add(1);
            if self.tick.is_multiple_of(BUSY_ANIM_DIV) {
                any_changed = true;
            }
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
                // Capture the title before the pane is removed, then notify.
                let title = self.panes[i].title_text();
                self.notify(crate::notify::NotifyKind::Exited, title, String::new());
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
        self.save_window_size_if_settled();

        if more_pending {
            // A pane is mid-flood with bytes still queued: poll again immediately
            // (winit still dispatches window events between ticks, so input stays
            // responsive) so the backlog catches up without a per-tick delay.
            event_loop.set_control_flow(ControlFlow::Poll);
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(POLL_MS),
            ));
        }
    }

    /// Persist the window size once resizing has settled (~400ms idle), so a
    /// drag produces a single write rather than one per frame.
    fn save_window_size_if_settled(&mut self) {
        if let Some(t) = self.resize_at {
            if t.elapsed() >= Duration::from_millis(400) {
                self.config.save();
                self.resize_at = None;
            }
        }
    }
}
