use crew_plugin::{Plugin, PluginCommand, PluginEvent};
use crew_render::CellView;
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use crate::chatlayout::{input_reduce, layout_cells, Message};

#[derive(Debug, PartialEq)]
pub enum HostAction {
    SpawnPane {
        command: String,
        args: Vec<String>,
        label: String,
    },
    SendPane {
        label: String,
        text: String,
    },
}

pub struct PollResult {
    pub changed: bool,
    pub actions: Vec<HostAction>,
}

pub fn classify(ev: &PluginEvent) -> Option<HostAction> {
    match ev {
        PluginEvent::SpawnPane {
            command,
            args,
            label,
        } => Some(HostAction::SpawnPane {
            command: command.clone(),
            args: args.clone(),
            label: label.clone(),
        }),
        PluginEvent::SendPane { label, text } => Some(HostAction::SendPane {
            label: label.clone(),
            text: text.clone(),
        }),
        _ => None,
    }
}

pub struct ChatPane {
    pub plugin: Plugin,
    pub channel: String,
    pub messages: Vec<Message>,
    pub input: String,
    pub connected: bool,
    /// Lines scrolled up from the live bottom (0 = following new messages).
    pub scroll: usize,
    /// A message was sent and no reply has arrived yet — drives the pane's
    /// indeterminate "thinking" progress sweep.
    awaiting: bool,
}

impl ChatPane {
    pub fn new(plugin: Plugin, channel: String) -> Self {
        ChatPane {
            plugin,
            channel,
            messages: Vec::new(),
            input: String::new(),
            connected: false,
            scroll: 0,
            awaiting: false,
        }
    }

    /// Whether the pane is awaiting a reply (busy), for the progress sweep.
    pub fn is_busy(&self) -> bool {
        self.awaiting
    }

    /// Scroll the message history by `delta` lines (positive = up/older),
    /// clamped to the available scrollback for the current width/height.
    pub fn scroll(&mut self, delta: i32, cols: u16, rows: u16) {
        let msg_rows = rows.saturating_sub(1) as usize;
        let max =
            crate::chatlayout::wrapped_line_count(&self.messages, cols).saturating_sub(msg_rows);
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
                    PluginEvent::Message { sender, text, .. } => {
                        self.awaiting = false; // a reply landed
                        self.messages.push(Message { sender, text });
                        if self.messages.len() > 500 {
                            let drain = self.messages.len() - 500;
                            self.messages.drain(..drain);
                        }
                    }
                    PluginEvent::Error { .. } => {
                        self.connected = false;
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

    /// Render the channel as CellView cells.
    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        layout_cells(
            &self.messages,
            &self.input,
            cols,
            rows,
            self.scroll,
            self.connected,
        )
    }

    /// Handle a winit key event. Returns [`ChatAction::Close`] when the user asks
    /// to close the pane (Escape) — mirroring the Far/Settings panes, which a chat
    /// pane previously lacked, leaving `/crew` only closable via the Cmd+W chord.
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

/// What a key press means to a chat pane. Extracted from `on_key` as a pure,
/// testable seam (winit's `KeyEvent` is `#[non_exhaustive]` and hard to build).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ChatInput {
    Close,
    Char(char),
    Enter,
    Backspace,
    Ignore,
}

/// An action a chat pane asks the app to take after a key press.
pub(crate) enum ChatAction {
    /// Close this pane (Escape).
    Close,
}

/// Classify a key press for a chat pane. Only presses act; Escape closes.
pub(crate) fn chat_key(logical: &Key, pressed: bool) -> ChatInput {
    if !pressed {
        return ChatInput::Ignore;
    }
    match logical {
        Key::Named(NamedKey::Escape) => ChatInput::Close,
        Key::Named(NamedKey::Enter) => ChatInput::Enter,
        Key::Named(NamedKey::Backspace) => ChatInput::Backspace,
        Key::Named(NamedKey::Space) => ChatInput::Char(' '),
        Key::Character(s) => s.chars().next().map_or(ChatInput::Ignore, ChatInput::Char),
        _ => ChatInput::Ignore,
    }
}

#[cfg(test)]
#[path = "chat_tests.rs"]
mod tests;
