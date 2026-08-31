use crew_plugin::{AgentInfo, Plugin};

use crate::chatflow::ActiveAgent;
use crate::chatlayout::Message;

pub use crate::chatevents::HostAction;

pub struct ChatPane {
    /// This pane's animated footer numbers. Per-pane, not global: two chat
    /// panes each sweep their own cost and token totals (see [`crate::readout`]).
    pub(crate) readouts: crate::readout::Readouts,
    pub plugin: Plugin,
    pub channel: String,
    pub messages: Vec<Message>,
    pub input: String,
    pub connected: bool,
    /// The agents the plugin can route to (name/role/model), for the roster row.
    pub agents: Vec<AgentInfo>,
    /// Lines scrolled up from the live bottom (0 = following new messages).
    pub scroll: usize,
    /// A message was sent and no reply has arrived yet — drives the pane's
    /// indeterminate "thinking" progress sweep. Cleared by
    /// `chatsettle::absorb_message` when a reply lands.
    pub(crate) awaiting: bool,
    /// The agents currently thinking (from `Activity` events): each with who
    /// handed it the work and when it started — several at once during a
    /// parallel /fan. Drives the live activity row (accessors in `chatflow`).
    pub(crate) active: Vec<ActiveAgent>,
    /// Session-wide approximate token spend (from `Stats` events), for the
    /// header's running cost meter.
    pub(crate) tokens: u64,
    /// Session prompt/completion token split and micro-USD cost, from
    /// turn-level `Stats` events (same cadence as `tokens`).
    pub(crate) tok_in: u64,
    pub(crate) tok_out: u64,
    pub(crate) cost_microusd: u64,
    /// Completed turns (turn-level `Stats` events), for the header.
    pub(crate) turns: u64,
    /// Per-agent totals from reply-level `Stats` events: name → (replies,
    /// total ms) — the roster chips show `n× avg` from these.
    pub(crate) agent_stats: std::collections::HashMap<String, (u32, u64)>,
    /// Each agent's latest real prompt size in tokens (its live context fill,
    /// from reply-level `Stats.ctx`) — the pulse lanes' ctx meter.
    pub(crate) ctx: std::collections::HashMap<String, u64>,
    /// Messages that arrived while scrolled up — the `↓ N new` pill. Cleared
    /// when the view returns to the live bottom.
    pub(crate) unread: usize,
    /// Hop timings observed live from activity/reply events — the pulse
    /// block's lane sparklines and turn waterfall (see `chatpulse`).
    pub(crate) pulse: crate::chatpulse::Pulse,
    /// The @file mention popup while one is being typed (see `chatmention`).
    pub(crate) mention: Option<crate::chatmention::MentionState>,
    /// The leading `/command` or `@agent` palette while one is open (see
    /// `chatpalette`). Mutually exclusive with `mention` by construction.
    pub(crate) palette: Option<crate::chatpalette::PaletteState>,
    /// The Ctrl+R reverse history-search popup while open (see
    /// `chathistsearch`). Takes keys before the palette and mention popups.
    pub(crate) histsearch: Option<crate::chathistsearch::HistSearch>,
    /// The Cmd+F/Ctrl+F transcript find popup while open (see `chatfind`).
    /// Mutually exclusive with `histsearch` — opening either closes the other.
    pub(crate) find: Option<crate::chatfind::ChatFind>,
    /// The masked provider-key prompt while one is open (see `keyentry`).
    /// Modal: it takes every key before the palette, the mention popup and the
    /// pane's own handling.
    pub(crate) keyentry: Option<crate::keyentry::KeyEntry>,
    /// An in-flight OpenRouter browser sign-in (see `oauth`). Dropping the
    /// receiver is what cancels the worker thread (its send then fails), so
    /// it is cleared exactly where the user DISMISSES the prompt — Escape and
    /// submit here, both through [`Self::cancel_oauth`], and closing the pane,
    /// which takes it down with everything else. Not when the prompt is merely
    /// hidden: see `close_hidden_keyentry`.
    pub(crate) oauth: Option<std::sync::mpsc::Receiver<crate::oauth::OauthOutcome>>,
    /// When true, show raw message text instead of markdown rendering.
    /// Toggled with Ctrl+Shift+M; not persisted.
    pub(crate) show_source: bool,
    /// When true, each transcript message renders its header line plus only
    /// the first body line, with a muted ` … +N` suffix noting how many
    /// lines are hidden (see `chatmsgs::View`). Toggled with Ctrl+O; not
    /// persisted. Orthogonal to `show_source` — both can be on at once.
    pub(crate) compact_view: bool,
    /// The live /crew swarm-run block (from `HivePlan`/`Hive` events); folded
    /// into a transcript message when the run ends (see `chatswarm`).
    pub(crate) swarm: Option<crate::chatswarm::SwarmStatus>,
    /// Text typed and submitted while the crew was busy: held here instead of
    /// sent immediately, flushed one at a time as each turn settles (see
    /// `chatqueue` for the indicator; the flush itself is in `poll` below,
    /// since it needs `plugin`/`awaiting`). `/stop` bypasses this queue
    /// entirely (it must reach the broker mid-run to cancel).
    pub(crate) queued: std::collections::VecDeque<String>,
    /// The watched repo's current branch, mirrored in from the sidebar's
    /// GitWatch each poll tick (`poll_panes`) — the summary footer shows it
    /// and must never run git itself on the winit thread.
    pub(crate) git_branch: Option<String>,
    /// A `/model all <slug>` the app should add to the recents list. Set on
    /// submit, drained by the poll — the command itself still goes to the
    /// broker untouched, this is only the app-side note.
    pub(crate) pending_recent: Option<String>,
    /// How many messages have been folded away in total, so the marker can
    /// report the running count instead of whatever the last pass dropped.
    pub(crate) folded: usize,
    /// A drafted plan is waiting for yes or no. Set by `PluginEvent::Plan`;
    /// while it is true the composer's Enter and Esc mean approve and discard,
    /// sent as the bare words the broker's deterministic plan gate matches
    /// (`/approve` and `/reject` retired as commands).
    pub(crate) plan_pending: bool,
    /// Where this pane's broker operates, mirrored in from the owning `Pane`
    /// each poll tick alongside `git_branch` — for the same reason: the footer
    /// shows it and the winit thread must never call `current_dir()` per frame
    /// to find out. `~`-abbreviated at mirror time, not at render time.
    pub(crate) cwd: Option<String>,
    /// Background task ids running in the broker right now, oldest first, fed
    /// by `PluginEvent::Task`. The footer shows these so the pane REPORTS what
    /// is in flight instead of the user having to ask — which is why `/tasks`
    /// no longer exists. Ids (not just a count) because `/stop #n` takes one.
    pub(crate) running_tasks: Vec<u64>,
    /// Agents mid-reply: one provisional card each, accumulating `Delta` text
    /// (most recently updated LAST — `absorb_delta` moves the card it touches
    /// to the end, which is how the overflow tail finds the newest without a
    /// timestamp field).
    ///
    /// NEVER stored in `messages`: the transcript holds only settled replies,
    /// so `/export`, `/restore`, the session log and the swarm fold are
    /// correct here by construction instead of each needing its own filter.
    pub(crate) streaming: Vec<Message>,
    /// The last per-reply `Stats` usage — `(agent, tok_in, tok_out,
    /// cost_microusd)` — waiting for its `Message` to land. The broker emits
    /// each reply's stat immediately before the reply itself (relay and fan
    /// alike), so one slot is enough; `chatsettle::absorb_message` drains it
    /// into the settled row's `usage`, but only when that row's sender is the
    /// stashed agent — adjacency is broker behavior, not a contract. Only set
    /// when the stat reported real usage (see `chatflow::absorb_stats`), so
    /// an all-zero stat never leaks a trailer.
    pub(crate) pending_reply_usage: Option<(String, u64, u64, u64)>,
    /// What this pane has already sent, for the composer's Up/Down recall
    /// (see `chathistory`). Not derived from `messages`: those are display
    /// records that fold, get restored from a session log and include replies,
    /// none of which is what the arrows should walk.
    pub(crate) history: crate::chathistory::History,
}

impl ChatPane {
    /// How many messages a pane keeps before folding the older ones away.
    pub(crate) const TRANSCRIPT_CAP: usize = 500;
    /// How far below the cap a fold takes the transcript, so folding happens
    /// once per this many messages rather than on every one.
    pub(crate) const FOLD_SLACK: usize = 50;

    /// The transcript text noting an Esc-interrupt — a single constant so the
    /// dedup check in [`Self::interrupt`] can compare against exactly what it
    /// pushes.
    pub(crate) const INTERRUPT_NOTE: &'static str = "\u{238b} interrupting \u{2014} sent /stop";
}

#[cfg(test)]
#[path = "chat_tests.rs"]
pub(crate) mod tests;
