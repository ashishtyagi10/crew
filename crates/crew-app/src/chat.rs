use crew_plugin::{AgentInfo, Plugin, PluginCommand, PluginEvent};
use crew_render::CellView;
use winit::event::KeyEvent;

use crate::chatflow::ActiveAgent;
use crate::chatinput::input_reduce;
use crate::chatkeys::{chat_key, ChatAction, ChatInput};
use crate::chatlayout::Message;

pub use crate::chatevents::{classify, HostAction, PollResult};

pub struct ChatPane {
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
    /// indeterminate "thinking" progress sweep.
    awaiting: bool,
    /// The agents currently thinking (from `Activity` events): each with who
    /// handed it the work and when it started — several at once during a
    /// parallel /fan. Drives the live activity row (accessors in `chatflow`).
    pub(crate) active: Vec<ActiveAgent>,
    /// Session-wide approximate token spend (from `Stats` events), for the
    /// header's running cost meter.
    pub(crate) tokens: u64,
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
}

impl ChatPane {
    pub fn new(plugin: Plugin, channel: String) -> Self {
        ChatPane {
            plugin,
            channel,
            messages: Vec::new(),
            input: String::new(),
            connected: false,
            agents: Vec::new(),
            scroll: 0,
            awaiting: false,
            active: Vec::new(),
            tokens: 0,
            turns: 0,
            agent_stats: std::collections::HashMap::new(),
            ctx: std::collections::HashMap::new(),
            unread: 0,
            pulse: crate::chatpulse::Pulse::new(),
            mention: None,
            palette: None,
            show_source: false,
            compact_view: false,
            swarm: None,
            queued: std::collections::VecDeque::new(),
        }
    }

    /// Append a local "agent smith" note to the transcript — composer intercepts
    /// (`/theme`, `/export`) and app-side command echoes (`/font`) share it.
    pub(crate) fn push_note(&mut self, text: String) {
        self.messages.push(Message {
            sender: "agent smith".into(),
            text,
            ts: chrono::Local::now().timestamp_millis().to_string(),
            meta: String::new(),
        });
    }

    /// Whether the pane is awaiting a reply (busy), for the progress sweep —
    /// either our own send is unanswered or agents are mid-turn.
    pub fn is_busy(&self) -> bool {
        self.awaiting || !self.active.is_empty() || self.swarm.is_some()
    }

    /// Drain plugin events; return PollResult with changed flag and any host actions.
    pub fn poll(&mut self) -> PollResult {
        let events = self.plugin.try_recv();
        if events.is_empty() {
            return PollResult {
                changed: false,
                actions: vec![],
            };
        }
        let mut actions = Vec::new();
        for ev in events {
            if let Some(action) = classify(&ev) {
                actions.push(action);
            } else {
                match ev {
                    PluginEvent::Ready { channels, .. } => {
                        self.connected = true;
                        if self.channel.is_empty() {
                            if let Some(ch) = channels.into_iter().next() {
                                self.channel = ch;
                            }
                        }
                    }
                    PluginEvent::Roster { agents } => {
                        self.agents = agents;
                    }
                    PluginEvent::Activity { agent, state, from } => {
                        self.absorb_activity(agent, &state, from);
                    }
                    PluginEvent::Stats {
                        tokens,
                        agent,
                        ms,
                        ctx,
                        ..
                    } => self.absorb_stats(tokens, agent, ms, ctx),
                    // Mid-reply token ticks fed only the retired per-agent tok
                    // ease; the summary footer reads settled per-turn `ctx`, so
                    // there's nothing live to update here now.
                    PluginEvent::StatsTick { .. } => {}
                    PluginEvent::Message {
                        sender,
                        text,
                        ts,
                        meta,
                        ..
                    } => {
                        self.awaiting = false; // a reply landed
                        self.note_reply(&sender);
                        if self.scroll > 0 {
                            self.unread += 1; // arrived out of view
                        }
                        self.push_capped(Message {
                            sender,
                            text,
                            ts,
                            meta,
                        });
                    }
                    PluginEvent::HivePlan { tasks } => self.absorb_hive_plan(tasks),
                    PluginEvent::Hive { event } => self.absorb_hive(&event),
                    PluginEvent::Error { .. } => {
                        self.fold_swarm();
                        self.connected = false;
                        self.flush_active_hops();
                    }
                    _ => {}
                }
            }
        }
        // Flush check: the busy→idle transition always arrives via one of the
        // events just processed (Activity idle, a swarm fold, or a Message
        // clearing `awaiting`), so it's enough to re-check here rather than
        // on every tick. One message per turn — the next flush waits for the
        // reply to *that* send to land and settle the pane again.
        //
        // Also gated on `connected`: a broker death mid-swarm/mid-active-hop
        // folds the swarm and flushes active hops in the same `Error` arm
        // that flips `connected` false — so `is_busy()` can go false in the
        // very same drain as the disconnect. Without this gate that race
        // pops the queue and calls `send_now` against the dead child right
        // here, silently dropping the text. Requiring `connected` closes it;
        // the queue then waits for a real reconnect (a fresh `Ready`) to
        // flush.
        if self.connected && !self.is_busy() {
            if let Some(text) = self.queued.pop_front() {
                self.send_now(text);
            }
        }
        PollResult {
            changed: true,
            actions,
        }
    }

    /// Push `m` onto the transcript, then trim from the front to the
    /// 500-message cap. Shared by every site that appends to `messages` (the
    /// plugin `Message` arm here, a folded swarm block in `chatswarm.rs`, and
    /// the Esc-interrupt note below) so the cap can't drift out of sync
    /// between them.
    pub(crate) fn push_capped(&mut self, m: Message) {
        self.messages.push(m);
        if self.messages.len() > 500 {
            let drain = self.messages.len() - 500;
            self.messages.drain(..drain);
        }
    }

    /// Send `text` to the broker on the pane's channel now, latching
    /// `awaiting` so the busy sweep runs until the reply lands. Shared by a
    /// direct send and a queue flush — both are "the broker gets this text
    /// now", just reached from different callers.
    fn send_now(&mut self, text: String) {
        let cmd = PluginCommand::Send {
            channel: self.channel.clone(),
            text,
        };
        match self.plugin.send(&cmd) {
            Ok(()) => self.awaiting = true, // wait for the reply
            Err(e) => eprintln!("crew-app: plugin send error: {e}"),
        }
    }

    /// Submit `text` to the broker as if it were typed in the composer: queued
    /// while the pane is busy (except `/stop`), sent immediately when idle —
    /// the same rule the composer's own Enter uses. Lets app-level commands
    /// that target this pane (e.g. the `/model` picker) reach the broker
    /// without a synthetic keystroke path.
    pub(crate) fn submit_command(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        if self.is_busy() && !crate::chatqueue::is_stop(&text) {
            self.queued.push_back(text);
        } else {
            self.send_now(text);
        }
    }

    /// The transcript text noting an Esc-interrupt — a single constant so the
    /// dedup check in [`Self::interrupt`] can compare against exactly what it
    /// pushes.
    const INTERRUPT_NOTE: &'static str = "\u{238b} interrupting \u{2014} sent /stop";

    /// Esc while the crew is busy and connected: cancel the in-flight run by
    /// sending `/stop` straight to the broker — bypassing the queue exactly
    /// like the composer's own `/stop` does (`send_now`, not `queued.push`),
    /// since it must reach the broker mid-turn to cancel it — and note the
    /// action in the transcript. Repeat Esc while still busy resends `/stop`
    /// (the broker's cancel is an idempotent `AtomicBool`) but the note is
    /// deduped: only pushed when the last transcript message isn't already
    /// this same note.
    fn interrupt(&mut self) {
        // Literal "/stop", not `chatmention::expand` — there's nothing to
        // expand in a fixed cancel token, so this stays allocation-free.
        self.send_now("/stop".to_string());
        let already_noted = self
            .messages
            .last()
            .is_some_and(|m| m.sender == "agent smith" && m.text == Self::INTERRUPT_NOTE);
        if already_noted {
            return;
        }
        if self.scroll > 0 {
            self.unread += 1;
        }
        self.push_capped(Message {
            sender: "agent smith".into(),
            text: Self::INTERRUPT_NOTE.into(),
            ts: String::new(),
            meta: String::new(),
        });
    }

    /// Render the channel as CellView cells: a status header, the agent roster
    /// (when known), role-styled message cards, and the input composer. Tiny
    /// panes (no room for a header) fall back to the plain body.
    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        crate::chatview::cells(self, cols, rows)
    }

    /// Handle a winit key event. Returns [`ChatAction::Close`] when the user asks
    /// to close the pane (Escape) — mirroring the Far/Settings panes. While the
    /// @file popup is open it gets keys first (Escape then closes the popup, not
    /// the pane). `shift` makes Enter insert a newline instead of sending.
    /// `cwd` roots mention scanning and expansion. (Ctrl+O's compact-transcript
    /// toggle is handled as a global intercept in `keys.rs`, not here.)
    pub fn on_key(
        &mut self,
        key: &KeyEvent,
        shift: bool,
        cwd: &std::path::Path,
    ) -> Option<ChatAction> {
        let k = chat_key(&key.logical_key, key.state.is_pressed(), shift);
        self.on_input(k, cwd)
    }

    /// Handle a decoded [`ChatInput`] — the testable half of [`on_key`], split
    /// out so the popup-vs-pane key routing can be exercised without
    /// constructing a winit `KeyEvent`.
    pub(crate) fn on_input(&mut self, k: ChatInput, cwd: &std::path::Path) -> Option<ChatAction> {
        // ORDER IS LOAD-BEARING: an open popup must get keys BEFORE the
        // `match k { Close/Up/Down/… }` block below, or Escape would close the
        // pane instead of the popup and arrows would never reach it. The
        // palette (leading token) and mention (mid-line) are mutually
        // exclusive, so their relative order is free — but both must precede
        // the pane's own key handling.
        if matches!(
            crate::chatpalette::popup_key(&mut self.palette, &mut self.input, &k),
            crate::chatpalette::PaletteKey::Consumed
        ) {
            return None;
        }
        if matches!(
            crate::chatmention::popup_key(&mut self.mention, &mut self.input, &k),
            crate::chatmention::MentionKey::Consumed
        ) {
            return None;
        }
        let (ch, enter, backspace) = match k {
            ChatInput::Close => {
                // Esc means "interrupt the running turn" while busy (mirrors
                // Codex/Claude Code); it only means "close the pane" once
                // idle. A dead connection can't be interrupted, so it falls
                // back to closing too — no write to a dead pipe.
                if self.is_busy() && self.connected {
                    self.interrupt();
                    return None;
                }
                return Some(ChatAction::Close);
            }
            ChatInput::Ignore | ChatInput::Up | ChatInput::Down => return None,
            ChatInput::Complete => {
                if let Some(done) = crate::chatcomplete::complete(&self.input, &self.agents) {
                    self.input = done;
                }
                return None;
            }
            ChatInput::Char(c) => (Some(c), false, false),
            ChatInput::Newline => (Some('\n'), false, false),
            ChatInput::Enter => (None, true, false),
            ChatInput::Backspace => (None, false, true),
        };
        if let Some(text) = input_reduce(&mut self.input, ch, enter, backspace) {
            self.scroll = 0; // sending snaps back to the live bottom
            if text.trim() == "/exit" {
                return Some(ChatAction::Close); // close the pane, like Escape
            }
            if crate::chatexport::intercept(self, &text) {
                return None; // answered locally (e.g. /export)
            }
            match crate::chattheme::intercept(self, &text) {
                // A switch must also be persisted app-side, or it silently
                // reverts on restart.
                crate::chattheme::ThemeIntercept::Switched => {
                    return Some(ChatAction::PersistTheme)
                }
                crate::chattheme::ThemeIntercept::Handled => return None,
                crate::chattheme::ThemeIntercept::NotTheme => {}
            }
            if crate::chatcompact::intercept(self, &text) {
                return None; // answered locally (/compact folds away older messages)
            }
            // `/font` needs the renderer, so the app runs it (and echoes the
            // status back here) — sending it to the broker did nothing.
            if let Some(arg) = crate::chatfont::parse(&text) {
                return Some(ChatAction::Font(arg));
            }
            if !text.is_empty() {
                // Echo the user's own prompt into the transcript, mirroring how
                // agent replies are appended in `poll` (the `PluginEvent::Message`
                // arm). Without this only replies were ever added, so the pane
                // showed output with no matching input. Echo the RAW typed text,
                // not `expanded`: mention expansion appends whole file bodies
                // meant for the broker, which don't belong in the display.
                // Scroll was already snapped to 0 above, so this lands in view.
                self.push_capped(Message {
                    sender: "user".into(),
                    text: text.clone(),
                    ts: chrono::Local::now().timestamp_millis().to_string(),
                    meta: String::new(),
                });
                let expanded = crate::chatmention::expand(&text, cwd);
                // Busy: queue instead of writing to a broker that's still
                // mid-turn — except `/stop`, which must reach it immediately
                // to cancel. Idle: send straight away, as before.
                if self.is_busy() && !crate::chatqueue::is_stop(&text) {
                    self.queued.push_back(expanded);
                } else {
                    self.send_now(expanded);
                }
            }
        } else {
            // A Char/Backspace edit: sync the mention popup to the new input.
            crate::chatmention::after_edit(&mut self.mention, &self.input, || {
                crate::fileindex::scan(cwd)
            });
            crate::chatpalette::after_edit(&mut self.palette, &self.input, &self.agents);
        }
        None
    }
}

#[cfg(test)]
#[path = "chat_tests.rs"]
mod tests;
