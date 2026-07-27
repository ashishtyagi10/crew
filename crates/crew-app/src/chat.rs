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
    /// which is why `/approve` and `/reject` are no longer in the palette.
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
    /// What this pane has already sent, for the composer's Up/Down recall
    /// (see `chathistory`). Not derived from `messages`: those are display
    /// records that fold, get restored from a session log and include replies,
    /// none of which is what the arrows should walk.
    pub(crate) history: crate::chathistory::History,
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
            tok_in: 0,
            tok_out: 0,
            cost_microusd: 0,
            turns: 0,
            agent_stats: std::collections::HashMap::new(),
            ctx: std::collections::HashMap::new(),
            unread: 0,
            pulse: crate::chatpulse::Pulse::new(),
            mention: None,
            palette: None,
            keyentry: None,
            oauth: None,
            show_source: false,
            compact_view: false,
            swarm: None,
            queued: std::collections::VecDeque::new(),
            folded: 0,
            plan_pending: false,
            git_branch: None,
            cwd: None,
            pending_recent: None,
            running_tasks: Vec::new(),
            streaming: Vec::new(),
            history: crate::chathistory::History::default(),
        }
    }

    /// Drop everything that belonged to one broker process: what it had
    /// running, and any plan it was holding for an answer. Called on both
    /// edges — a new broker (`Ready`) and a lost one (`Error`) — because both
    /// invalidate the same state, and neither can be relied on to arrive.
    pub(crate) fn reset_broker_state(&mut self) {
        self.running_tasks.clear();
        self.plan_pending = false;
    }

    /// A background task started or ended in the broker. Kept as a list of
    /// ids, oldest first, because `/stop #n` names one — and an END for a task
    /// this pane never saw start (a broker that outlived a reconnect) is a
    /// no-op rather than a phantom or a panic.
    pub(crate) fn absorb_task(&mut self, id: u64, running: bool) {
        self.running_tasks.retain(|t| *t != id);
        if running {
            self.running_tasks.push(id);
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

    /// End an in-flight browser sign-in, and SAY SO. The single place a live
    /// flow is dismissed while its pane is still around.
    ///
    /// Dropping the receiver is what cancels it: the worker's `send` then
    /// fails and the outcome — possibly a key OpenRouter has already minted
    /// against the user's account — is discarded. That must never be silent,
    /// or the only trace left is a key the user has to find and revoke by
    /// hand. A no-op (and no note) when no sign-in was in flight.
    pub(crate) fn cancel_oauth(&mut self) {
        if self.oauth.take().is_some() {
            self.push_note("openrouter sign-in cancelled".into());
        }
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
                        // A fresh broker has nothing running and no plan
                        // waiting — and it numbers its tasks from 1 again, so
                        // anything left from the last one would both lie and
                        // collide.
                        self.reset_broker_state();
                        if self.channel.is_empty() {
                            if let Some(ch) = channels.into_iter().next() {
                                self.channel = ch;
                            }
                        }
                    }
                    PluginEvent::Roster { agents } => {
                        self.agents = agents;
                    }
                    PluginEvent::Task { id, running, .. } => self.absorb_task(id, running),
                    PluginEvent::Plan { pending } => self.plan_pending = pending,
                    PluginEvent::Activity { agent, state, from } => {
                        self.absorb_activity(agent, &state, from);
                    }
                    PluginEvent::Stats {
                        tokens,
                        agent,
                        ms,
                        ctx,
                        tok_in,
                        tok_out,
                        cost_microusd,
                        ..
                    } => self.absorb_stats(tokens, agent, ms, ctx, tok_in, tok_out, cost_microusd),
                    // Mid-reply token ticks fed only the retired per-agent tok
                    // ease; the summary footer reads settled per-turn `ctx`, so
                    // there's nothing live to update here now.
                    PluginEvent::StatsTick { .. } => {}
                    PluginEvent::Delta { agent, text } => self.absorb_delta(agent, text),
                    PluginEvent::Message {
                        sender,
                        text,
                        ts,
                        meta,
                        ..
                    } => {
                        self.settle_stream(&sender);
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
                        // Nothing survives the broker that was running it. A
                        // task that dies with its process never sends its end
                        // event, so without this the footer would offer
                        // `/stop #3` for a task that no longer exists —
                        // forever.
                        self.reset_broker_state();
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
        if self.messages.len() <= Self::TRANSCRIPT_CAP {
            return;
        }
        // Fold with a marker rather than draining in silence — the cap has
        // always been automatic; what it lacked was honesty.
        //
        // Down to CAP - SLACK in one batch, not to CAP: folding to exactly
        // the cap means the next message folds again, and every message
        // after that, each pass copying the whole transcript. One pass per
        // SLACK messages instead.
        let mut msgs = std::mem::take(&mut self.messages);
        if self.folded > 0 {
            // The leading marker is not one of the messages being counted.
            msgs.remove(0);
        }
        let (kept, total) = crate::chatcompact::compact_messages(
            msgs,
            Self::TRANSCRIPT_CAP - Self::FOLD_SLACK,
            self.folded,
        );
        self.messages = kept;
        self.folded = total;
    }

    /// How many messages a pane keeps before folding the older ones away.
    pub(crate) const TRANSCRIPT_CAP: usize = 500;
    /// How far below the cap a fold takes the transcript, so folding happens
    /// once per this many messages rather than on every one.
    pub(crate) const FOLD_SLACK: usize = 50;

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

    /// Every card the transcript should draw this frame. ONE source, so the
    /// scroll clamp, the scrollbar, the link hit-test and the unread pill can
    /// never disagree about what is on screen — the same reason `View` is
    /// threaded through all four render entry points.
    pub(crate) fn visible_messages(&self) -> Vec<&Message> {
        self.messages.iter().chain(self.streaming.iter()).collect()
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
        // The masked key prompt takes every key before anything else: it may
        // hold a half-typed secret, and letting a key leak past it to the
        // palette or the composer would be the one bug this feature exists
        // to avoid.
        if let Some(entry) = self.keyentry.as_mut() {
            match entry.key(&k) {
                crate::keyentry::KeyOutcome::Consumed => return None,
                crate::keyentry::KeyOutcome::Cancelled => {
                    self.keyentry = None;
                    self.cancel_oauth();
                    return None;
                }
                crate::keyentry::KeyOutcome::Submit(value) => {
                    let var = entry.var.clone();
                    self.keyentry = None;
                    // Answering by hand supersedes the browser: end that flow
                    // FIRST, so the note explaining the now-pointless tab
                    // reads before the one confirming the key.
                    self.cancel_oauth();
                    crate::chatkeystore::store_provider_key(self, &var, &value);
                    return None;
                }
            }
        }
        // ORDER IS LOAD-BEARING: an open popup must get keys BEFORE the
        // `match k { Close/Up/Down/… }` block below, or Escape would close the
        // pane instead of the popup and arrows would never reach it. The
        // palette (leading token) and mention (mid-line) are mutually
        // exclusive, so their relative order is free — but both must precede
        // the pane's own key handling.
        match crate::chatpalette::popup_key(&mut self.palette, &mut self.input, &k) {
            crate::chatpalette::PaletteKey::Consumed => return None,
            // A filled-but-not-run row: the input changed, so the palette has
            // to be re-synced against it exactly as a typed character would.
            // `/model` fills `/model `, which opens the model picker.
            crate::chatpalette::PaletteKey::Accepted => {
                self.sync_palette(cwd);
                return None;
            }
            // A picked row is a command to run: the palette is closed and the
            // input holds it, so re-enter our own Enter path (no recursion —
            // `self.palette` is `None` now).
            crate::chatpalette::PaletteKey::Submit => return self.on_input(ChatInput::Enter, cwd),
            crate::chatpalette::PaletteKey::Forward => {}
            crate::chatpalette::PaletteKey::NeedsKey(var) => {
                let mut entry = crate::keyentry::KeyEntry::new(var.clone());
                if var == crate::oauth::OPENROUTER_KEY_VAR {
                    // OpenRouter is the one provider with a real third-party
                    // OAuth flow. The paste prompt still opens underneath, so
                    // a failed browser launch is never a dead end.
                    self.oauth = crate::oauth::spawn();
                    entry.set_waiting(self.oauth.is_some());
                }
                self.keyentry = Some(entry);
                return None;
            }
        }
        if matches!(
            crate::chatmention::popup_key(&mut self.mention, &mut self.input, &k),
            crate::chatmention::MentionKey::Consumed
        ) {
            return None;
        }
        let (ch, enter, backspace) = match k {
            ChatInput::Close => {
                // A pending plan owns Esc: it is the foreground question, and
                // answering it is what the user means. Only with an empty
                // composer — half-typed text means they moved on, and Esc
                // should not silently throw away a plan behind it.
                if self.plan_pending && self.input.is_empty() {
                    self.plan_pending = false;
                    self.submit_command("/reject".to_string());
                    return None;
                }
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
            ChatInput::Ignore => return None,
            // No popup is open (both got these keys first, above), so the
            // arrows mean what they mean in every shell: walk what you already
            // sent. The palette is deliberately NOT re-synced from a recalled
            // line — it would open on any `/command` and then swallow the next
            // Up as popup navigation, which is the opposite of what the user
            // is in the middle of doing. Typing a character re-syncs it as
            // usual.
            ChatInput::Up => {
                self.history.prev(&mut self.input);
                return None;
            }
            ChatInput::Down => {
                self.history.next(&mut self.input);
                return None;
            }
            ChatInput::Complete => {
                if let Some(done) = crate::chatcomplete::complete(&self.input, &self.agents) {
                    self.input = done;
                }
                return None;
            }
            ChatInput::Char(c) => (Some(c), false, false),
            ChatInput::Newline => (Some('\n'), false, false),
            ChatInput::Enter => {
                // Enter on an empty composer answers the pending plan. With
                // text typed it sends that text as usual — the plan stays
                // pending, since the user plainly had something else to say.
                if self.plan_pending && self.input.is_empty() {
                    self.plan_pending = false;
                    self.submit_command("/approve".to_string());
                    return None;
                }
                (None, true, false)
            }
            ChatInput::Backspace => (None, false, true),
        };
        if let Some(text) = input_reduce(&mut self.input, ch, enter, backspace) {
            self.scroll = 0; // sending snaps back to the live bottom
                             // Every submitted line, whatever happens to it next: the ones
                             // answered locally (`/theme`, `/export`) and the ones that never
                             // reach the broker are exactly as worth recalling as the rest, so
                             // this records BEFORE any of the intercepts below return.
            self.history.record(&text);
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
            // `/font` needs the renderer, so the app runs it (and echoes the
            // status back here) — sending it to the broker did nothing.
            if let Some(arg) = crate::chatfont::parse(&text) {
                return Some(ChatAction::Font(arg));
            }
            // Note a `/model all <slug>` pick for the recents list — bare
            // bookkeeping, not a `ChatAction`: returning one here would end
            // the call and swallow the send below. The broker still gets the
            // command untouched; `poll` drains this into the config.
            if let Some(slug) = text.strip_prefix("/model all ") {
                let slug = slug.trim();
                if !slug.is_empty() && slug != "default" {
                    self.pending_recent = Some(slug.to_string());
                }
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
                let agent_names: Vec<String> = self.agents.iter().map(|a| a.name.clone()).collect();
                let expanded = crate::chatmention::expand(&text, cwd, &agent_names);
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
            // A Char/Backspace edit: the composer's text is the user's own
            // again, whether or not it started as a recalled line.
            self.history.edited();
            // Sync the mention popup to the new input.
            let agents = self.agents.clone();
            crate::chatmention::after_edit(&mut self.mention, &self.input, || {
                crate::chatmention::scan_entries(cwd, &agents)
            });
            self.sync_palette(cwd);
        }
        None
    }

    /// Re-sync the leading-token palette to the current input. Called after a
    /// character edit and after a palette row is accepted (`PaletteKey::
    /// Accepted`) — both change the input, and the palette must follow it.
    fn sync_palette(&mut self, cwd: &std::path::Path) {
        let agents = self.agents.clone();
        let current = crate::chatpalette::shared_model(&self.agents);
        crate::chatpalette::after_edit(&mut self.palette, &self.input, current.as_deref(), || {
            crate::chatmention::scan_entries(cwd, &agents)
        });
    }
}

#[cfg(test)]
#[path = "chat_tests.rs"]
pub(crate) mod tests;
