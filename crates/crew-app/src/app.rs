use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use winit::event::Modifiers;
use winit::window::Window;

use crate::config::CrewConfig;
use crate::grid::GridLayout;
use crate::inputbar::InputBar;
use crate::pane::Pane;
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
    /// Documents opened into windows of their own ([`crate::docwin`]). Not
    /// panes: a document window holds no grid, so none of the app's
    /// pane-shaped state applies to it.
    pub(crate) docs: Vec<crate::docwin::DocWindow>,
    /// The first canvas of the process. The launch note, the crash report and
    /// the upgrade migrations are about this LAUNCH, not about this window, so
    /// a second canvas does not repeat them (see [`crate::canvas`]).
    pub(crate) first: bool,
    /// This canvas asked for another window (`Cmd+N`). A window can only be
    /// created from a callback holding the active event loop, so the ask is a
    /// flag `canvas` answers on the next tick.
    pub(crate) want_window: bool,
    /// Panes a `/restore` found belonging to windows this canvas is not —
    /// handed to [`crate::canvas`], which opens a window for each group.
    pub(crate) pending_windows: Vec<Vec<crate::sessionsave::SavedPane>>,
    /// This canvas's window was closed. Closing the LAST one quits; closing
    /// any other just removes it.
    pub(crate) closing: bool,
    /// How many panes the OTHER canvases are holding, stamped by
    /// [`crate::canvas`] before each event. Read by the quit guard, which
    /// speaks for the whole app.
    pub(crate) other_panes: usize,
    /// Documents asked for a window since the last tick. A window can only be
    /// created from a winit callback holding the ACTIVE event loop, and the
    /// key handler is not one — so the ask is queued here and drained in
    /// `about_to_wait`.
    pub(crate) pending_docs: Vec<std::path::PathBuf>,
    pub(crate) panes: Vec<Pane>,
    pub(crate) focused: usize,
    /// Which pane the focus brackets were last drawn around, and the timeline
    /// they are travelling on. Focus is reassigned from a dozen places (chords,
    /// clicks, close, restore); diffing it once per frame in `build_frame`
    /// catches every one of them without each having to remember to stamp a
    /// timeline.
    /// Wheel-gesture speed, so a flick crosses a log and a nudge reads it
    /// ([`crate::scrollboost`]).
    pub(crate) scroll_boost: crate::scrollboost::Boost,
    /// A command that ends panes, waiting to be run a second time
    /// ([`crate::confirm`]).
    pub(crate) pending: crate::confirm::Pending,
    /// A multi-line paste held for a second Cmd+V, because the program in
    /// the pane would run it line by line ([`crate::pastesafe`]).
    pub(crate) held_paste: crate::pastesafe::Held,
    /// `/diff`'s background read of the working tree ([`crate::diffjob`]),
    /// drained once a tick.
    pub(crate) diff_job: crate::diffjob::DiffJob,
    /// Git status for each pane's directory, refreshed off-thread and read
    /// by the cards' badges ([`crate::gitfleet`]).
    pub(crate) git_fleet: crate::gitfleet::GitFleet,
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
    /// What has been typed into the `/keys` overlay to filter it. Cleared
    /// when the overlay closes — a filter that survives a close is one you
    /// meet again with no memory of having set it.
    pub(crate) help_filter: String,
    /// Whether crew holds the OS window focus, or `None` before the platform
    /// has said either way. Ambient motion — the only motion that asks for
    /// frames nothing else needed — stops when the answer is `Some(false)`: a
    /// window you are not looking at repaints for nobody. `None` counts as
    /// focused, because a window that has just opened is, and not every
    /// platform sends `Focused(true)` to say so.
    pub(crate) win_focus: Option<bool>,
    /// Whether the focused pane is zoomed to fill the content area.
    pub(crate) zoomed: bool,
    /// What focus mode has held back since it was entered (see
    /// [`crate::focusmode`]). Reset every time the mode is entered, reported
    /// and cleared on the way out.
    pub(crate) held: crate::focusmode::Held,
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
    /// How far back the sidebar LOG is scrolled — 0 follows the newest line
    /// (a window onto the buffer; the rest used to be reachable only via `/log`).
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
    /// Panes closed this session that crew could bring back — `/reopen`
    /// (Cmd+Shift+T) pops the most recent. See [`crate::reopen`].
    pub(crate) closed: crate::reopen::ClosedStack,
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
    /// kicked off; `None` before the first `/model` picker open and after it lands.
    pub(crate) model_fetch: Option<std::sync::mpsc::Receiver<Vec<crew_hive::catalog::LiveModel>>>,
    /// Kicked off this process — a picker reopening must not spawn a second worker.
    pub(crate) model_fetch_started: bool,
    /// The weather worker in flight, and when the next fetch is due
    /// (`anim::now_ms`; 0 = now). See `navweather::tick_weather`.
    pub(crate) weather_fetch: Option<std::sync::mpsc::Receiver<Option<crate::navweather::Weather>>>,
    pub(crate) weather_next: u64,
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
    /// Crew's own furniture, in physical px: the rects a sheer window keeps
    /// solid whatever has focus (the input bar, the left nav). Rebuilt each
    /// frame from the layout and handed to the renderer — transparency is for
    /// the canvas, not for the bar you type into.
    pub(crate) solid_chrome: Vec<[f32; 4]>,
}

impl CrewApp {
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
/// depends on `/broadcast` state.
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

/// The appearance guard lives in [`crate::appearanceguard`]; re-exported
/// here because every test already reaches for it as `app::theme_test_guard`.
#[cfg(test)]
pub(crate) use crate::appearanceguard::{
    holds_appearance_guard, motion_test_guard, theme_test_guard, ThemeGuard, THEME_LOCK,
};

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
