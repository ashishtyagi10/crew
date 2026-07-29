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
        if !self.had_restorable {
            use crate::pane::PaneContent;
            self.had_restorable = self.panes.iter().any(|p| match &p.content {
                PaneContent::Terminal(_) | PaneContent::Far(_) => true,
                PaneContent::Chat(_) => p.label.as_deref() == Some("crew"),
                _ => false,
            });
        }
        if self.window.is_none() {
            return;
        }

        // Live OpenRouter enrichment: kick off the background fetch the
        // first time a `/model` picker opens (guarded so it fires once per
        // process), then drain whatever's landed. `try_recv` on a `None`
        // handle or an empty/disconnected channel is a cheap no-op, so this
        // costs nothing on every other tick — including forever offline or
        // with no key, where `spawn()` never even returned a receiver.
        if !self.model_fetch_started && self.model_picker_open() {
            // Only latch the guard when `spawn()` actually returned a
            // receiver. If the picker opens before the login-shell key probe
            // lands (up to its 3s timeout), the key lookup falls back to the
            // process env and `spawn()` returns `None` on a GUI launch —
            // latching anyway would disable enrichment for the rest of the
            // session even once the probe finishes and a key is available.
            self.model_fetch = crate::modelfetch::spawn();
            self.model_fetch_started = self.model_fetch.is_some();
        }
        let mut model_fetch_landed = false;
        if let Some(rx) = &self.model_fetch {
            if let Ok(models) = rx.try_recv() {
                crate::modelpick::set_live(models);
                model_fetch_landed = true;
            }
        }

        let oauth_landed = self.drain_oauth();

        // Random theme mode: rotate on its 10-minute clock. Cheap + lock-free.
        // Seeds `any_changed` so a rotation repaints every pane in the new theme.
        // A rotation changes the active theme app-wide, so re-apply the themeable
        // accent (it follows the theme when the user hasn't pinned one) exactly
        // like every other theme-change path — otherwise the accent stays frozen
        // on the previous theme after the first rotation.
        let now_ms = crate::chattime::unix_now_ms();
        let rotated = crew_theme::tick_random(now_ms);
        if rotated {
            crate::palette::set_accent(self.config.accent_rgb());
        }
        // Font rotation: same 10-minute clock as the theme rotation. The pick
        // updates the renderer only — config.font_family stays pinned.
        self.tick_font_rotation(now_ms);
        // Then the theme's own font, if the theme changed since last tick.
        // Deliberately LAST: both hang off the same 10-minute clock, so on a
        // rotation tick both fire and the theme's font must land on top. This
        // one place catches every theme-change path (`/theme`, the picker,
        // Ctrl+Shift+L, the rotation tick) rather than four call sites each
        // remembering to do it.
        self.tick_theme_font();
        //
        // Drain EVERY pane each tick. A `for` loop (not `any()`/`fold`) so all
        // panes are polled for their side effects — `any()` would short-circuit
        // and starve later panes when an earlier one has output.
        let mut any_changed = rotated || model_fetch_landed || oauth_landed;
        // Set when any pane still has buffered PTY output past this tick's read
        // budget. We then keep the loop hot (ControlFlow::Poll) so a flood drains
        // quickly across ticks instead of trickling one budget per 16 ms — while
        // each tick stays bounded, so input and rendering never stall.
        let mut more_pending = false;
        let mut collected_actions = Vec::new();
        // Status flashes from Far panes whose command finished this tick
        // (surfaced after the loop to avoid fighting the panes borrow).
        let mut far_statuses: Vec<String> = Vec::new();
        // `FarAction`s a Far pane's `poll_ops` produced this tick (e.g. a
        // finished remote download's `Open`), paired with the pane index —
        // applied after the loop via the shared `apply_far_action` (see
        // `faraction.rs`), same reason as `far_statuses`.
        let mut far_actions: Vec<(usize, crate::farpane::FarAction)> = Vec::new();
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
                    // A `/model` pick this pane just noted (composer popup or
                    // input-bar picker) → move it to the front of the recents
                    // list, cap it, republish the process-global `rows()`
                    // reads, and persist. Drained here rather than via a
                    // `ChatAction` because that would end `on_input`'s call
                    // and swallow the send — this is pure app-side
                    // bookkeeping alongside the pane's other per-tick drains.
                    if let Some(slug) = c.pending_recent.take() {
                        let recents = &mut self.config.model_recents;
                        recents.retain(|s| *s != slug);
                        recents.insert(0, slug);
                        recents.truncate(crate::modelpick::MAX_RECENTS);
                        crate::modelpick::set_recents(recents.clone());
                        self.config.save();
                    }
                    result.changed
                }
                PaneContent::Swarm(s) => s.poll(),
                // A Far pane changes when its command-line command finishes
                // (panels reload) or its `!` AI ask lands/errors — either
                // becomes a status flash.
                PaneContent::Far(f) => {
                    let mut changed = false;
                    if let Some(msg) = f.poll_cmd() {
                        far_statuses.push(msg);
                        changed = true;
                    }
                    if let Some(msg) = f.poll_ask() {
                        far_statuses.push(msg);
                        changed = true;
                    }
                    if let Some(action) = f.poll_ops() {
                        // Deferred past the borrow of `p`/`f` — apply_far_action
                        // takes `&mut self` (e.g. to `close_pane` or flash a
                        // status), which can't happen while this pane is
                        // borrowed. `i` is the pane being polled, standing in
                        // for the key path's `focused`.
                        far_actions.push((i, action));
                        changed = true;
                    }
                    // Auto-upload: a watched temp file's mtime advancing means
                    // the OS-default app saved an edit — push it back to the
                    // remote. Runs every tick regardless of `is_busy()` (the
                    // event loop wakes on its own `POLL_MS` timer even while
                    // idle, so a watch is never starved), but only actually
                    // kicks off an upload when no other rclone op is pending.
                    if let Some(action) = f.poll_watches() {
                        far_actions.push((i, action));
                        changed = true;
                    }
                    changed
                }
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
        // Inter-pane `ask`: serve socket requests + advance live asks against
        // the output the pane loop just captured this tick.
        any_changed |= self.pump_asks(now_ms);
        // Broadcast asks aggregate silently (no injection here), so they don't
        // force a repaint — advance them off the same captured output.
        self.tick_castings(now_ms);
        if self.sidebar.refresh(&self.cwd) {
            any_changed = true;
        }
        // Mirror the watched repo's branch into every smith pane: the summary
        // footer shows `model | branch | …` and must never run git itself on
        // this thread — the sidebar's GitWatch already polls off-thread.
        let branch = self.sidebar.branch().map(str::to_string);
        // Same trip, same reason, for the working directory the footer shows:
        // the pane's own spawn dir when it has one, else the app's. Abbreviated
        // here rather than at render time — `cwd::display` reads $HOME, and the
        // winit thread should not be doing that per frame either.
        let app_cwd = self.cwd.clone();
        for p in &mut self.panes {
            let dir = crate::cwd::display(p.dir.as_deref().unwrap_or(&app_cwd));
            if let PaneContent::Chat(c) = &mut p.content {
                if c.git_branch != branch {
                    c.git_branch.clone_from(&branch);
                    any_changed = true;
                }
                if c.cwd.as_deref() != Some(dir.as_str()) {
                    c.cwd = Some(dir);
                    any_changed = true;
                }
            }
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
        for (idx, action) in far_actions.drain(..) {
            self.apply_far_action(action, idx);
        }
        // Quiet auto-update: fire the same worker the manual /update uses,
        // silently, shortly after launch and then six-hourly. Restart stays
        // manual — an install only parks the nav-legend reminder. The silent
        // state is poll-driven immediately below in the same frame (first drain
        // is empty, no side effects beyond spawning the worker).
        if self.autoupdate.take_due(Instant::now()) {
            self.start_auto_update();
        }
        // Drive the background self-update: animate its card and dismiss it when
        // done. The new binary applies on `/restart` — Crew does not restart itself.
        if self.update.is_some() {
            let tick = self.poll_update(Instant::now());
            any_changed |= tick.redraw;
        }
        // Keep the nav legend blinking while a parked install's reminder is
        // still inside its pulse window — settles to a steady accent (and
        // stops costing frames) once the window elapses.
        if let Some((_, at)) = &self.parked_update {
            if crate::restartnote::animating(crate::anim::now_ms(), *at) {
                any_changed = true;
            }
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
            || self.focus_anim.live(crate::anim::now_ms())
            || self.ghosts.iter().any(|g| g.live(crate::anim::now_ms()))
            || self.zoom_anim.live(crate::anim::now_ms())
            || self.panes.iter().any(|p| match &p.content {
                PaneContent::Chat(c) => c.readouts.any_live(crate::anim::now_ms()),
                _ => false,
            })
            || self
                .panes
                .iter()
                .any(|p| crate::paneview::spawn_timeline(p).live(crate::anim::now_ms()))
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

    /// Land any finished OpenRouter browser sign-in, in EVERY pane — not just
    /// the focused one. Window focus is not pane focus, so alt-tabbing to the
    /// browser was never the problem; deliberately switching to another pane
    /// mid-flow was. Polling only `self.focused` left the started-it pane's
    /// receiver undrained: the user approved and nothing happened, a "saved ·
    /// pinned" note appeared out of nowhere on returning minutes later, and
    /// closing that pane silently discarded a key OpenRouter had already
    /// issued. Each outcome goes to the pane that started the flow, which is
    /// also the pane holding the prompt the note belongs under.
    ///
    /// `try_recv` on a `None` handle or an empty channel is a cheap no-op, so
    /// this costs nothing on every tick where no sign-in is in flight.
    /// Returns whether anything landed (a repaint is due).
    pub(crate) fn drain_oauth(&mut self) -> bool {
        self.drain_oauth_into(crew_plugin::credentials::path().as_deref())
    }

    /// [`Self::drain_oauth`] against an explicit credentials path — the
    /// testable half, resolved once here rather than per pane. A test can then
    /// prove that a completed sign-in really is STORED (and that a dismissed
    /// one stores nothing) against a temporary file, without writing to the
    /// user's real credentials or touching a process-global (see
    /// `chatkeystore`'s own `_at` seam, and its doc for why).
    pub(crate) fn drain_oauth_into(&mut self, creds: Option<&std::path::Path>) -> bool {
        let mut landed = false;
        for p in &mut self.panes {
            let PaneContent::Chat(pane) = &mut p.content else {
                continue;
            };
            let Some(outcome) = pane.oauth.as_ref().and_then(|rx| rx.try_recv().ok()) else {
                continue;
            };
            match outcome {
                crate::oauth::OauthOutcome::Key(key) => {
                    crate::chatkeystore::store_provider_key_in(
                        pane,
                        creds,
                        crate::oauth::OPENROUTER_KEY_VAR,
                        &key,
                    );
                    pane.keyentry = None;
                }
                crate::oauth::OauthOutcome::Failed(why) => {
                    // `why` can carry text the callback supplied. It is
                    // clamped where it is produced; clamped again here so a
                    // future producer cannot widen a pane note by accident.
                    let why = crate::oauth::short_reason(&why);
                    pane.push_note(format!("openrouter sign-in failed: {why}"));
                    if let Some(e) = pane.keyentry.as_mut() {
                        e.set_waiting(false);
                    }
                }
            }
            pane.oauth = None;
            landed = true;
        }
        landed
    }

    /// Whether a `/model` picker is open right now, on either of its two
    /// surfaces: the input bar's value picker (`self.input`, a plain text
    /// prefix — it has no persisted "open" flag, it's recomputed from the
    /// text each render) or a chat pane's composer popup (`ChatPane::palette`,
    /// which does persist while typing). Checked every tick rather than at
    /// every place a palette can open, so the once-per-process fetch trigger
    /// is a single guarded call site here.
    fn model_picker_open(&self) -> bool {
        self.input.focused && self.input.text.trim_start().starts_with("/model")
            || self.panes.iter().any(|p| {
                matches!(&p.content, PaneContent::Chat(c)
                    if c.palette.as_ref().is_some_and(|pl| pl.kind == crate::chatpalette::Kind::Model))
            })
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
