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
        }
    }

    /// Whether the pane is awaiting a reply (busy), for the progress sweep —
    /// either our own send is unanswered or agents are mid-turn.
    pub fn is_busy(&self) -> bool {
        self.awaiting || !self.active.is_empty()
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
                        self.messages.push(Message {
                            sender,
                            text,
                            ts,
                            meta,
                        });
                        if self.messages.len() > 500 {
                            let drain = self.messages.len() - 500;
                            self.messages.drain(..drain);
                        }
                    }
                    PluginEvent::Error { .. } => {
                        self.connected = false;
                        self.flush_active_hops();
                    }
                    _ => {}
                }
            }
        }
        PollResult {
            changed: true,
            actions,
        }
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
    /// the pane). `cwd` roots mention scanning and expansion.
    pub fn on_key(&mut self, key: &KeyEvent, cwd: &std::path::Path) -> Option<ChatAction> {
        let k = chat_key(&key.logical_key, key.state.is_pressed());
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
            ChatInput::Close => return Some(ChatAction::Close),
            ChatInput::Ignore | ChatInput::Up | ChatInput::Down => return None,
            ChatInput::Complete => {
                if let Some(done) = crate::chatcomplete::complete(&self.input, &self.agents) {
                    self.input = done;
                }
                return None;
            }
            ChatInput::Char(c) => (Some(c), false, false),
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
            if !text.is_empty() {
                let cmd = PluginCommand::Send {
                    channel: self.channel.clone(),
                    text: crate::chatmention::expand(&text, cwd),
                };
                match self.plugin.send(&cmd) {
                    Ok(()) => self.awaiting = true, // wait for the reply
                    Err(e) => eprintln!("crew-app: plugin send error: {e}"),
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
