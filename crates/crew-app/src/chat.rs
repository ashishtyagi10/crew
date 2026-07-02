use crew_plugin::{AgentInfo, Plugin, PluginCommand, PluginEvent};
use crew_render::CellView;
use winit::event::KeyEvent;

use crate::chatflow::ActiveAgent;
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
    /// The agents currently thinking (from `Activity` events): each with who
    /// handed it the work and when it started — several at once during a
    /// parallel /fan. Drives the live activity row.
    active: Vec<ActiveAgent>,
    /// Session-wide approximate token spend (from `Stats` events), for the
    /// header's running cost meter.
    pub(crate) tokens: u64,
    /// Messages that arrived while scrolled up — the `↓ N new` pill. Cleared
    /// when the view returns to the live bottom.
    pub(crate) unread: usize,
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
            unread: 0,
        }
    }

    /// Whether the pane is awaiting a reply (busy), for the progress sweep —
    /// either our own send is unanswered or agents are mid-turn.
    pub fn is_busy(&self) -> bool {
        self.awaiting || !self.active.is_empty()
    }

    /// The live status label: the thinking agent's name (one active) or a
    /// `N working` count (parallel fan), with the oldest elapsed seconds.
    pub(crate) fn active_status(&self) -> Option<(String, u64)> {
        let secs = self
            .active
            .iter()
            .map(|a| a.since.elapsed().as_secs())
            .max()?;
        match &self.active[..] {
            [one] => Some((one.name.clone(), secs)),
            many => Some((format!("{} working", many.len()), secs)),
        }
    }

    /// Names of every agent currently thinking (roster highlights them all).
    pub(crate) fn active_names(&self) -> Vec<&str> {
        self.active.iter().map(|a| a.name.as_str()).collect()
    }

    /// The live activity entries, for the pane's interaction row.
    pub(crate) fn active_agents(&self) -> &[ActiveAgent] {
        &self.active
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
                        match (state.as_str(), agent.is_empty()) {
                            ("thinking", false) => {
                                if !self.active.iter().any(|a| a.name == agent) {
                                    self.active.push(ActiveAgent {
                                        name: agent,
                                        from,
                                        since: std::time::Instant::now(),
                                    });
                                }
                            }
                            ("idle", false) => self.active.retain(|a| a.name != agent),
                            // An empty-agent idle (turn over) clears everyone.
                            _ => self.active.clear(),
                        }
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
                        self.active.clear();
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
