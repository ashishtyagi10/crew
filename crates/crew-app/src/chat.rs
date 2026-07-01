use crew_plugin::{AgentInfo, Plugin, PluginCommand, PluginEvent};
use crew_render::CellView;
use winit::event::KeyEvent;

use crate::chatkeys::{chat_key, ChatAction, ChatInput};
use crate::chatlayout::{input_reduce, Message};

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
    /// The agent currently thinking (from `Activity` events) and when it
    /// started, for the header's live `agent · elapsed` status.
    active: Option<(String, std::time::Instant)>,
    /// Session-wide approximate token spend (from `Stats` events), for the
    /// header's running cost meter.
    pub(crate) tokens: u64,
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
            active: None,
            tokens: 0,
        }
    }

    /// Whether the pane is awaiting a reply (busy), for the progress sweep —
    /// either our own send is unanswered or an agent is mid-turn.
    pub fn is_busy(&self) -> bool {
        self.awaiting || self.active.is_some()
    }

    /// The agent currently thinking and for how many seconds, if any.
    pub(crate) fn active_status(&self) -> Option<(&str, u64)> {
        self.active
            .as_ref()
            .map(|(a, t)| (a.as_str(), t.elapsed().as_secs()))
    }

    /// Rows consumed above the message body: the status header, plus the agent
    /// roster row when agents are known and the pane is tall enough.
    pub(crate) fn top_rows(&self, rows: u16) -> u16 {
        match rows {
            0..=2 => 0,
            3 => 1,
            _ if self.agents.is_empty() => 1,
            _ => 2,
        }
    }

    /// Scroll the message history by `delta` lines (positive = up/older),
    /// clamped to the available scrollback for the current width/height.
    pub fn scroll(&mut self, delta: i32, cols: u16, rows: u16) {
        // The header/roster rows and the input row sit outside the message area.
        let top = self.top_rows(rows);
        let msg_rows = rows.saturating_sub(top + 1) as usize;
        // The card view (normal panes) and the plain fallback (tiny panes)
        // wrap to different line counts; clamp against whichever is shown.
        let total = if top == 0 {
            crate::chatlayout::wrapped_line_count(&self.messages, cols)
        } else {
            crate::chatmsgs::card_line_count(&self.messages, cols)
        };
        let max = total.saturating_sub(msg_rows);
        let next = self.scroll as i64 + delta as i64;
        self.scroll = next.clamp(0, max as i64) as usize;
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
                    PluginEvent::Activity { agent, state } => {
                        self.active = (state == "thinking" && !agent.is_empty())
                            .then(|| (agent, std::time::Instant::now()));
                    }
                    PluginEvent::Stats { tokens, .. } => {
                        self.tokens = self.tokens.saturating_add(tokens);
                    }
                    PluginEvent::Message {
                        sender,
                        text,
                        ts,
                        meta,
                        ..
                    } => {
                        self.awaiting = false; // a reply landed
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
                        self.active = None;
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
    /// to close the pane (Escape) — mirroring the Far/Settings panes.
    pub fn on_key(&mut self, key: &KeyEvent) -> Option<ChatAction> {
        let (ch, enter, backspace) = match chat_key(&key.logical_key, key.state.is_pressed()) {
            ChatInput::Close => return Some(ChatAction::Close),
            ChatInput::Ignore => return None,
            ChatInput::Char(c) => (Some(c), false, false),
            ChatInput::Enter => (None, true, false),
            ChatInput::Backspace => (None, false, true),
        };
        if let Some(text) = input_reduce(&mut self.input, ch, enter, backspace) {
            self.scroll = 0; // sending snaps back to the live bottom
            if !text.is_empty() {
                let cmd = PluginCommand::Send {
                    channel: self.channel.clone(),
                    text,
                };
                match self.plugin.send(&cmd) {
                    Ok(()) => self.awaiting = true, // wait for the reply
                    Err(e) => eprintln!("crew-app: plugin send error: {e}"),
                }
            }
        }
        None
    }
}

#[cfg(test)]
#[path = "chat_tests.rs"]
mod tests;
