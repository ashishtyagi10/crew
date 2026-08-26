use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use winit::event::Modifiers;
use winit::window::Window;

use crate::config::CrewConfig;
use crate::grid::GridLayout;
use crate::inputbar::InputBar;
use crate::pane::Pane;
use crate::session::grid_for;
use crate::statspane::StatsPane;
use crew_render::Renderer;
use crew_term::GridSize;

/// Fallback grid size when the GPU cell size is not yet known (zero).
pub(crate) const FALLBACK_SIZE: GridSize = GridSize { cols: 80, rows: 24 };
pub(crate) const POLL_MS: u64 = 16;
/// The gutter between every pane card and its neighbours, in logical px.
///
/// A function rather than a constant since the Density setting owns it (see
/// [`crate::density`]): render and hit-testing both call this, so they cannot
/// disagree about where a card's edge is.
pub(crate) fn gap() -> f32 {
    crate::density::gap()
}

#[derive(Default)]
pub struct CrewApp {
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) renderer: Option<Renderer>,
    pub(crate) panes: Vec<Pane>,
    pub(crate) focused: usize,
    /// Which pane the focus brackets were last drawn around, and the timeline
    /// they are travelling on. Focus is reassigned from a dozen places (chords,
    /// clicks, close, restore); diffing it once per frame in `build_frame`
    /// catches every one of them without each having to remember to stamp a
    /// timeline.
    /// Cards that have been dismissed but are still collapsing. Bounded by
    /// their own timelines and pruned every frame (see [`crate::ghost`]).
    pub(crate) ghosts: Vec<crate::ghost::Ghost>,
    /// Grid reflow glide (see [`crate::glide`]): when `build_frame` last
    /// stepped pane rects, and whether any pane is still travelling — the
    /// redraw-scheduling flag `wants_animation_frame` reads.
    pub(crate) glide_prev_ms: u64,
    pub(crate) glide_active: bool,
    /// Theme-switch crossfade (see [`crate::themefade`]): the last theme id
    /// drawn, and the old-frame melt running when it changes. `None` until
    /// first frame.
    pub(crate) theme_seen: Option<crew_theme::ThemeId>,
    pub(crate) theme_fade_anim: crate::ease::Timeline,
    /// The light/dark scheme last pushed to DECSET-2031 terminals (see
    /// [`crate::schemepush`]). `None` until the first poll tick latches it.
    pub(crate) scheme_pushed: Option<bool>,
    /// Zoom transition: the rect the focused pane occupied when zoom was
    /// toggled, and the timeline it is travelling on. A zoom that cut straight
    /// to full size lost the connection between the tile and the thing that
    /// filled the screen.
    pub(crate) zoom_from: Option<crate::layout::Rect>,
    pub(crate) zoom_anim: crate::ease::Timeline,
    pub(crate) focus_drawn: usize,
    /// Where the spotlight travelled from — the pane whose content dims as
    /// the focused one brightens (see [`crate::spotlight`]).
    pub(crate) focus_prev: usize,
    pub(crate) focus_anim: crate::ease::Timeline,
    /// One-shot CRT ignition sweep: on the phosphor themes a freshly focused
    /// frame starts corner-node hot and decays to `border_focused` (see
    /// [`crate::panecardglow`]). Default is settled, so paper themes and cold
    /// starts never animate it.
    pub(crate) ignite_anim: crate::ease::Timeline,
    /// LRU of pane indices: which panes are full tiles vs. minimized.
    pub(crate) grid: GridLayout,
    pub(crate) mods: Modifiers,
    pub(crate) cursor: (f32, f32),
    /// Whether the pointer is inside the window at all.
    ///
    /// `cursor` keeps the last position it saw, which is the right answer for
    /// hit-testing a click but the wrong one for hover: with the pointer
    /// parked over another app, a toast stack whose hold was decided from a
    /// stale coordinate would hold forever. Cleared on `CursorLeft`.
    pub(crate) cursor_in: bool,
    /// Sub-line scroll remainder, in lines. Trackpads emit many small pixel
    /// deltas; we accumulate the fractional part here so slow scrolling adds up
    /// instead of each tick rounding to zero and being lost.
    pub(crate) scroll_accum: f32,
    /// Last resolved (first_word, verdict) — see [`Self::check_command`].
    pub(crate) cmd_cache: Option<(String, crate::cmdcheck::Verdict)>,
    pub(crate) config: CrewConfig,
    pub(crate) sidebar: Box<StatsPane>,
    /// Resolves each terminal pane's foreground PID to a command name for its
    /// title (e.g. `claude`), refreshed ~1×/s.
    pub(crate) procnames: crate::procname::ProcNames,
    /// `/font random` rotation state (pool cache + 10-minute clock).
    pub(crate) font_rotate: crate::fontrotate::FontRotate,
    pub(crate) input: InputBar,
    /// Animation frame counter, advanced while the welcome screen is showing.
    pub(crate) tick: u64,
    /// Whether the keybindings help overlay is showing, and how far down its
    /// list it has been scrolled (see [`crate::help`]).
    pub(crate) help_open: bool,
    pub(crate) help_scroll: usize,
    /// Whether crew holds the OS window focus, or `None` before the platform
    /// has said either way. Ambient motion — the only motion that asks for
    /// frames nothing else needed — stops when the answer is `Some(false)`: a
    /// window you are not looking at repaints for nobody. `None` counts as
    /// focused, because a window that has just opened is, and not every
    /// platform sends `Focused(true)` to say so.
    pub(crate) win_focus: Option<bool>,
    /// Whether the focused pane is zoomed to fill the content area.
    pub(crate) zoomed: bool,
    /// Last OS window title set, to avoid redundant `set_title` calls.
    pub(crate) win_title: String,
    /// Mirror input to every terminal pane (tmux-style synchronized input).
    pub(crate) broadcast: bool,
    /// Time, pane index and run length of the last left click — the state
    /// behind the click *run* (single → word → line, see [`crate::select`]).
    pub(crate) last_click: Option<(Instant, usize, u8)>,
    /// A fold toggle the last mouse press landed on — `(pane index, absolute
    /// row)` — waiting for its release. The toggle fires on RELEASE, and only
    /// when the gesture stayed a plain click: a drag-selection started on a
    /// folded card must not expand it mid-gesture (see `chatfold`).
    pub(crate) fold_click: Option<(usize, u16)>,
    /// In-progress mouse drag selection over any pane, if any.
    pub(crate) drag: Option<crate::select::Drag>,
    /// The pane whose right-border scroll gutter is in hand (see
    /// [`crate::panegutter`]).
    pub(crate) gutter_drag: Option<usize>,
    /// The sidebar's resize edge is in hand (see [`crate::navresize`]).
    pub(crate) nav_drag: bool,
    /// The cursor shape currently set on the window, so a pointer move that
    /// changes nothing costs no platform call (see [`crate::pointer`]).
    pub(crate) cursor_icon: winit::window::CursorIcon,
    /// A card picked up by its legend row and not yet dropped (see
    /// [`crate::panedrag`]). Mutually exclusive with `drag`: the legend row is
    /// the one row of a card that holds nothing to select.
    pub(crate) card_drag: Option<crate::panedrag::CardDrag>,
    /// Active text selection over a non-terminal pane (chat/settings/etc.),
    /// which lack alacritty's grid model. Persists after the drag so `Cmd+C`
    /// can copy it; cleared by the next press or a scroll. See [`crate::gridsel`].
    pub(crate) cell_sel: Option<crate::gridsel::CellSel>,
    /// Last `/find` term, so repeating it walks to the next older match.
    pub(crate) last_find: Option<String>,
    /// The last `/findall` term — repeating it cycles through matching panes.
    pub(crate) last_findall: Option<String>,
    /// Crew's working directory: shown in the input-bar legend and used as the
    /// start directory for new shells. Moved by typing `cd` in the input bar.
    pub(crate) cwd: PathBuf,
    /// The directory before the last change, so `cd -` can toggle back.
    pub(crate) prev_cwd: PathBuf,
    /// When the window was last resized; drives a debounced save of its size.
    pub(crate) resize_at: Option<Instant>,
    /// A note to flash on the FIRST rendered frame rather than when it was
    /// decided. Status messages expire after three seconds, and a cold launch
    /// takes far longer than that to reach its first frame — so a note set
    /// during `resumed()` (the version-change announcement is the only one)
    /// would expire unseen on exactly the launch it exists for.
    pub(crate) pending_note: Option<String>,
    /// Transient status message + when it was set, shown on the input bar.
    pub(crate) status: Option<(String, Instant)>,
    /// Ring buffer of recent status messages, shown as the live LOG section in
    /// the left nav (newest last). Capped at [`crate::status::LOG_CAP`].
    pub(crate) log: Vec<crate::applog::LogEntry>,
    /// How far back the sidebar LOG is scrolled — 0 follows the newest line.
    /// The log is a five-row window onto a hundred buffered entries, and
    /// until now the other ninety-five were only reachable through `/log`.
    pub(crate) log_back: usize,
    /// Channel background threads stream LOG lines through; drained once per
    /// poll tick into [`Self::set_status_level`]. See [`crate::applog`].
    pub(crate) applog: crate::applog::AppLog,
    /// Notification system: throttles + records pane events (command finished,
    /// bell, output pattern match, pane exit) surfaced via the LOG + input bar.
    pub(crate) notifier: crate::notify::Notifier,
    /// Transient toast cards at the top-right of the canvas — the loud surface
    /// for notify events and errors (the input-bar flash is the quiet one).
    pub(crate) toasts: crate::toast::Toasts,
    /// When quit was last pressed with panes open, for the confirm-to-quit window.
    pub(crate) quit_armed: Option<Instant>,
    /// Whether a restorable pane (shell / Far / crew chat / file viewer)
    /// ever existed this session — gates the quit snapshot so a pane-less
    /// run can't wipe a saved `/restore` session.
    pub(crate) had_restorable: bool,
    /// Saved-session shell count for the welcome screen's `/restore` hint
    /// (seeded at startup, cleared once `/restore` spends the snapshot).
    pub(crate) restore_hint: Option<usize>,
    /// In-progress background self-update (`/update`): drives the left-nav UPDATE
    /// card and the auto-restart. `None` when no update is running.
    pub(crate) update: Option<crate::update::UpdateState>,
    /// Pure-timing scheduler for the quiet background update check: 30 s after
    /// launch, then every 6 h. See [`crate::autoupdate`].
    pub(crate) autoupdate: crate::autoupdate::AutoUpdate,
    /// Version + parked-at (on the `anim` clock) of the most recently
    /// installed-but-not-yet-running update, set the moment any run (silent
    /// or manual) reaches `Installed`. Drives the blinking nav-legend
    /// reminder (see [`crate::restartnote`]); the next `/update` restarts
    /// straight into it.
    pub(crate) parked_update: Option<(String, u64)>,
    /// In-flight `?` ask (AI command suggestion) on a worker thread. `None`
    /// when idle. See [`crate::askbar`].
    pub(crate) ask: Option<crate::askbar::Ask>,
    /// Inter-pane `ask` IPC endpoint (the Unix socket, on its own thread).
    /// `None` if the socket couldn't bind. See [`crate::ipc`], [`crate::askpump`].
    pub(crate) ipc: Option<crate::ipc::IpcHandle>,
    /// Live inter-pane asks: each target pane's liveness state + the channel
    /// its verdict is sent back on when it resolves.
    pub(crate) pending_asks: Vec<(
        crate::askwait::PendingAsk,
        std::sync::mpsc::Sender<crate::ipc_types::Reply>,
    )>,
    /// Live broadcast asks (`crew ask --all` / `--any`): each fans one question
    /// across a set of panes and aggregates their verdicts. See [`crate::askcast`].
    pub(crate) castings: Vec<crate::askcast::Casting>,
    /// The live OpenRouter enrichment fetch (`crate::modelfetch`), once
    /// kicked off. `None` before the first `/model` picker open, and again
    /// forever after that fetch lands or fails — `try_recv` on an empty,
    /// disconnected channel is a cheap no-op either way.
    pub(crate) model_fetch: Option<std::sync::mpsc::Receiver<Vec<crew_hive::catalog::LiveModel>>>,
    /// Whether the enrichment fetch has been kicked off this process — a
    /// picker reopening must not spawn a second worker.
    pub(crate) model_fetch_started: bool,
    /// When the user last typed, clicked, or scrolled (on the `anim` clock).
    /// Gates blocked-pane auto-focus: focus is never stolen while the user is
    /// actively driving some other pane (see [`crate::blocked`]).
    pub(crate) last_input_ms: u64,
    /// Blocked-on-a-human episode tracking + check throttle (see
    /// [`crate::blocked`]): which panes are waiting, and which have already
    /// had their one auto-focus for the current episode.
    pub(crate) blocked: crate::blocked::BlockedState,
    /// Once-a-minute clock behind the todo due-toast check (see
    /// [`crate::todopane::store::take_due`], driven from `poll_panes`).
    pub(crate) todo_due: crate::todopane::DueTicker,
    /// Where the modern backdrop's gradient wash sits on its orbit (see
    /// [`crate::washphase`]) — advanced only by the frames activity is
    /// already drawing.
    pub(crate) wash: crate::washphase::WashPhase,
    /// Where that orbit is CENTRED: glided toward the focused card, so the
    /// page's light gathers where the work is (see [`crate::washfocus`]).
    pub(crate) wash_focus: crate::washfocus::WashFocus,
}

impl CrewApp {
    pub(crate) fn current_grid(renderer: &Renderer) -> GridSize {
        let (cell_w, cell_h) = renderer.cell_size();
        if cell_w > 0.0 && cell_h > 0.0 {
            let (sw, sh) = renderer.surface_size();
            grid_for(sw, sh, cell_w, cell_h)
        } else {
            FALLBACK_SIZE
        }
    }

    /// Close pane at `idx`.  Returns `true` if the app should exit.
    pub fn close_pane(&mut self, idx: usize) -> bool {
        if idx < self.panes.len() {
            // Record where the card was before the pane is gone: everything
            // downstream reads `panes`, so the pane cannot linger — only the
            // frame it leaves behind can.
            let p = &self.panes[idx];
            self.ghosts.push(crate::ghost::Ghost::new(
                p.rect,
                p.title_text(),
                crate::ghost::Exit::Closed,
                crate::anim::now_ms(),
            ));
            self.panes.remove(idx);
            self.grid.on_close(idx);
        }
        // Closing a pane returns to the grid; never linger zoomed on it.
        self.zoomed = false;
        if self.panes.is_empty() {
            // No panel selected → focus returns to the input bar; reset modes.
            self.focused = 0;
            self.input.focused = true;
            self.broadcast = false;
            self.input.broadcast = false;
            return false;
        }
        self.focused = self.focused.min(self.panes.len() - 1);
        // Never let the clamp land focus on a pane minimized into the nav —
        // reconcile_grid would silently restore it. Prefer a visible pane;
        // with none left, the input bar takes focus and the pane stays tucked.
        if self.panes[self.focused].hidden {
            match self.nearest_visible(self.focused) {
                Some(i) => self.focused = i,
                None => self.input.focused = true,
            }
        }
        false
    }

    /// The non-hidden pane index nearest to `idx`, if any pane is visible.
    pub(crate) fn nearest_visible(&self, idx: usize) -> Option<usize> {
        (0..self.panes.len())
            .filter(|&i| !self.panes[i].hidden)
            .min_by_key(|&i| i.abs_diff(idx))
    }

    /// Keep the grid LRU in step with `self.panes` and the current focus. Adds
    /// any visible pane index not yet tracked (newly spawned), drops hidden and
    /// stale indices, and marks the focused pane most-recently-active. Called
    /// once per frame from `build_frame`.
    pub(crate) fn reconcile_grid(&mut self) {
        let n = self.panes.len();
        // Keyboard-focusing a hidden pane restores it — the one rule that makes
        // every focus path (nav-row click, Cmd+N, spawn) a restore path. The
        // input bar holding focus means no pane is active, so nothing restores.
        if !self.input.focused {
            if let Some(p) = self.panes.get_mut(self.focused) {
                // Restoring re-stamps the birth clock, so a pane coming back
                // out of the nav assembles exactly as a new one does — it is,
                // as far as the grid is concerned, arriving.
                if p.hidden {
                    p.born_ms = crate::anim::now_ms();
                }
                p.hidden = false;
            }
        }
        // Hidden panes leave the grid without reindexing — a hide keeps the
        // panes vec intact, unlike a close. Also drops any stale index past the
        // end (defensive; close_pane already fixes the common case via on_close).
        let panes = &self.panes;
        self.grid
            .retain(|i| panes.get(i).is_some_and(|p| !p.hidden));
        for idx in 0..n {
            if !self.panes[idx].hidden
                && !self.grid.full().contains(&idx)
                && !self.grid.minimized().contains(&idx)
            {
                self.grid.add(idx);
            }
        }
        if n > 0 {
            self.grid.touch(self.focused.min(n - 1));
        }
    }

    /// Focus the most-recently-pushed pane and move keyboard focus off the input bar.
    pub(crate) fn focus_new_pane(&mut self) {
        self.focused = self.panes.len().saturating_sub(1);
        self.input.focused = false;
    }

    /// Handle a submitted input line: `/command`s are run; everything else is
    /// written (with a newline) to the focused Terminal pane. Returns `true` if the
    /// app should exit (e.g. `/exit`).
    pub(crate) fn submit_input(&mut self, line: String) -> bool {
        if line.is_empty() {
            return false;
        }
        if let Some(cmd) = slash_command(&line) {
            return self.run_slash_command(cmd);
        }
        // `!cmd` runs a shell command in its own pane (like `/run`), regardless of
        // which pane is focused — a quick `ls`/`git status` without leaving the
        // agent pane you're driving.
        if let Some(cmd) = bang_command(&line) {
            if cmd.is_empty() {
                self.set_status("usage: !<command>");
            } else {
                self.run_in_pane(cmd);
            }
            return false;
        }
        // `*text` broadcasts one line to every terminal pane, explicitly — the
        // bar's replacement for depending on Cmd+S broadcast mode.
        if let Some(cmd) = star_command(&line) {
            if cmd.is_empty() {
                self.set_status("usage: *<text> — sends to every terminal");
            } else if self.write_terminal_targets(&submit_bytes(cmd), true) == 0 {
                self.set_status("no terminals to broadcast to");
            }
            return false;
        }
        // `??question` asks the AI to explain the focused pane's output; the
        // answer opens in the zoomed markdown viewer. Checked before `?` —
        // qmark_command would read `??x` as an ask for "?x".
        if let Some(question) = crate::askbar::explain_command(&line) {
            self.start_explain(question);
            return false;
        }
        // `?query` asks the AI for a shell command (à la Warp AI); the reply
        // lands back in the input bar, ready to edit or Enter.
        if let Some(query) = crate::askbar::qmark_command(&line) {
            if query.is_empty() {
                self.set_status("usage: ?<what you want> — ask ai for a command");
            } else {
                self.start_ask(query);
            }
            return false;
        }
        // `cd` in the input bar moves Crew's working directory, not the terminal's.
        if self.try_change_dir(&line) {
            return false;
        }
        match crate::route::route_bare(self.focused_target(), &self.check_command(&line)) {
            crate::route::BareRoute::TypeInto(_) => {
                // The focused idle shell receives the line as keystrokes.
                if self.write_terminal_targets(&submit_bytes(&line), false) == 0 {
                    self.set_status("no shell here — press Cmd+T to open one");
                }
            }
            crate::route::BareRoute::Spawn => self.run_in_pane(&line),
            crate::route::BareRoute::BuiltinHint(b) => {
                self.set_status(format!(
                    "{b} is a shell builtin — run it inside a shell pane"
                ));
            }
            crate::route::BareRoute::UnknownHint => {
                self.set_status(format!("not a command — !{line} runs it in a pane anyway"));
            }
        }
        false
    }

    /// The focused pane as routing sees it: `IdleShell` only for a visible
    /// terminal whose shell owns the prompt (`foreground_pid()` is `None`).
    /// Hidden panes are not "in the main area", so they never receive text.
    pub(crate) fn focused_target(&self) -> crate::route::Target {
        if let Some(p) = self.panes.get(self.focused) {
            if !p.hidden {
                if let crate::pane::PaneContent::Terminal(t) = &p.content {
                    if t.pty.foreground_pid().is_none() {
                        return crate::route::Target::IdleShell(self.focused);
                    }
                }
            }
        }
        crate::route::Target::Other
    }

    /// Resolve `line`'s first word, memoized — the palette preview re-checks on
    /// every keystroke and only the first word matters, so argument typing must
    /// not re-stat the PATH.
    pub(crate) fn check_command(&mut self, line: &str) -> crate::cmdcheck::Verdict {
        let word = crate::cmdcheck::first_word(line);
        if let (Some(w), Some((cached_w, v))) = (&word, &self.cmd_cache) {
            if w == cached_w {
                return v.clone();
            }
        }
        let v = crate::cmdcheck::resolve(line, &crate::cmdcheck::effective_path());
        if let Some(w) = word {
            self.cmd_cache = Some((w, v.clone()));
        }
        v
    }

    /// Set (or, when `name` is empty, clear) the focused pane's title override.
    pub(crate) fn name_focused_pane(&mut self, name: &str) {
        if let Some(p) = self.panes.get_mut(self.focused) {
            p.name = (!name.is_empty()).then(|| name.to_string());
            self.redraw();
        } else {
            self.set_status("no pane to name");
        }
    }

    /// Toggle the window's maximized state and persist it.
    pub(crate) fn toggle_maximize(&mut self) {
        if let Some(w) = &self.window {
            let m = !w.is_maximized();
            w.set_maximized(m);
            self.config.maximized = m;
        }
        self.config.save();
    }

    pub(crate) fn toggle_sidebar(&mut self) {
        self.config.show_nav = !self.config.show_nav;
        self.config.save();
        self.redraw();
    }

    pub(crate) fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    /// The CRT style that should be active right now, if any: the user's
    /// `/crt` override if set, otherwise the active theme's own style (the
    /// phosphor themes each ship one). `/crt on` over a paper theme still
    /// works — it falls back to `CrtStyle::DEFAULT` since paper themes carry
    /// no style of their own. Read every frame so it tracks live theme changes.
    pub(crate) fn effective_crt(&self) -> Option<crew_theme::CrtStyle> {
        match self.config.crt {
            Some(false) => None,
            Some(true) => crew_theme::theme()
                .crt
                .or(Some(crew_theme::CrtStyle::DEFAULT)),
            None => crew_theme::theme().crt,
        }
    }
}

/// If `line` is a `/command`, return the trimmed command name; else `None`.
pub(crate) fn slash_command(line: &str) -> Option<&str> {
    line.strip_prefix('/').map(str::trim)
}

/// If `line` is a `!command`, return the trimmed command (empty when just `!`);
/// else `None`. The command runs in its own pane via [`CrewApp::run_in_pane`].
pub(crate) fn bang_command(line: &str) -> Option<&str> {
    line.strip_prefix('!').map(str::trim)
}

/// If `line` is a `*text` broadcast, return the trimmed payload (empty when
/// just `*`); else `None`. The payload is sent to EVERY terminal pane —
/// broadcast is an explicit prefix, not a mode, so nothing else the bar does
/// depends on Cmd+S state.
pub(crate) fn star_command(line: &str) -> Option<&str> {
    line.strip_prefix('*').map(str::trim)
}

/// Bytes to write when submitting an input-bar line to a terminal: the line
/// followed by a carriage return (0x0d) — the same byte a real Enter sends. A
/// trailing line feed (0x0a) is the Shift+Enter "soft return", which agent CLIs
/// (Claude/codex) treat as "insert a newline, keep editing", leaving the text
/// sitting highlighted in their input box instead of being submitted.
pub(crate) fn submit_bytes(line: &str) -> Vec<u8> {
    let mut bytes = line.as_bytes().to_vec();
    bytes.push(b'\r');
    bytes
}

/// Serialises tests that mutate crew-theme's process-global state (`CURRENT`,
/// the random-rotation atomics): several files across this crate exercise
/// `/theme` behaviour (chattheme.rs, toggles.rs, spawn.rs, config.rs) and
/// would otherwise race under the default parallel test runner. Mirrors the
/// `guard()` used by crew-theme's own tests.
#[cfg(test)]
static THEME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Holds the lock, pins a known theme, and RESTORES whatever was active on drop.
///
/// The restore is the load-bearing half; the pin only decides what the first taker in a process
/// sees. No test here distinguishes them — a test that tried was found to pass with the pin
/// removed, so it was deleted rather than kept as decoration.
///
/// The lock alone was not enough. Serialising theme-touching tests stops them racing, but it
/// leaves whichever theme the last one published in place for everybody after — and a test that
/// compares against a derived colour (`chatink` floors every ink for contrast against the card)
/// then passes or fails depending on what ran before it. Three markdown colour tests went from
/// green to red overnight with no code change that way, and reproduced under
/// `--test-threads=1`, which is what ruled a race out and pointed here.
#[cfg(test)]
pub(crate) struct ThemeGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    prev: crew_theme::ThemeId,
}

#[cfg(test)]
impl Drop for ThemeGuard {
    fn drop(&mut self) {
        crew_theme::set_theme(self.prev);
    }
}

#[cfg(test)]
pub(crate) fn theme_test_guard() -> ThemeGuard {
    let lock = THEME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let prev = crew_theme::current_id();
    // The default. A test wanting another theme still sets one after taking the guard; this
    // only decides what a test that never mentions a theme gets, which used to be "whatever the
    // previous test left behind".
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    ThemeGuard { _lock: lock, prev }
}

/// Serialises tests that touch the process-global motion level
/// (`motion::LEVEL`): the animation tests across this crate (app_tests,
/// the chatmsgs caret, ghost.rs, readout.rs, paneview_tests, motion.rs)
/// each pin a level and then read it back through timelines; unguarded they
/// race under the parallel runner — an `Off` window opened by one test makes
/// another's just-started timeline instant (`every_animation_terminates` was
/// the observed flake).
///
/// SHARES [`theme_test_guard`]'s lock rather than adding a second one:
/// `apply_config` mutates both globals in one call, so the apply_config
/// tests (holding only the theme guard) flip the motion level too — a
/// separate lock would let a guarded animation test race exactly that. One
/// consequence: a test must take one guard or the other, NEVER both (the
/// mutex is not reentrant).
#[cfg(test)]
pub(crate) fn motion_test_guard() -> ThemeGuard {
    theme_test_guard()
}

#[cfg(test)]
mod unit_tests {
    use super::star_command;

    #[test]
    fn star_command_strips_the_prefix() {
        assert_eq!(star_command("* ls -la"), Some("ls -la"));
        assert_eq!(star_command("*ls"), Some("ls"));
        assert_eq!(star_command("*"), Some(""));
        assert_eq!(star_command("ls *"), None);
    }
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
