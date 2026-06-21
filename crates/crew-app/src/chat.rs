use crew_plugin::{Plugin, PluginCommand, PluginEvent};
use crew_render::CellView;
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use crate::chatlayout::{input_reduce, layout_cells, Message};

pub struct ChatPane {
    pub plugin: Plugin,
    pub channel: String,
    pub messages: Vec<Message>,
    pub input: String,
    pub connected: bool,
}

impl ChatPane {
    pub fn new(plugin: Plugin, channel: String) -> Self {
        ChatPane {
            plugin,
            channel,
            messages: Vec::new(),
            input: String::new(),
            connected: false,
        }
    }

    /// Drain plugin events; return true if any state changed (caller should redraw).
    pub fn poll(&mut self) -> bool {
        let events = self.plugin.try_recv();
        if events.is_empty() {
            return false;
        }
        for ev in events {
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
                    self.messages.push(Message { sender, text });
                    if self.messages.len() > 500 {
                        let drain = self.messages.len() - 500;
                        self.messages.drain(..drain);
                    }
                }
                PluginEvent::Error { .. } => {
                    self.connected = false;
                }
            }
        }
        true
    }

    /// Render the channel as CellView cells.
    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        layout_cells(&self.messages, &self.input, cols, rows)
    }

    /// Handle a winit key event: translate to (char, enter, backspace) and reduce.
    pub fn on_key(&mut self, key: &KeyEvent) {
        if !key.state.is_pressed() {
            return;
        }
        let (ch, enter, backspace) = match &key.logical_key {
            Key::Named(NamedKey::Enter) => (None, true, false),
            Key::Named(NamedKey::Backspace) => (None, false, true),
            Key::Named(NamedKey::Space) => (Some(' '), false, false),
            Key::Character(s) => (s.chars().next(), false, false),
            _ => (None, false, false),
        };
        if let Some(text) = input_reduce(&mut self.input, ch, enter, backspace) {
            if !text.is_empty() {
                let cmd = PluginCommand::Send {
                    channel: self.channel.clone(),
                    text,
                };
                if let Err(e) = self.plugin.send(&cmd) {
                    eprintln!("crew-app: plugin send error: {e}");
                }
            }
        }
    }
}
