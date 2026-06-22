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
        }
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
        layout_cells(&self.messages, &self.input, cols, rows, self.scroll)
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
            self.scroll = 0; // sending snaps back to the live bottom
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_spawn_pane_returns_host_action() {
        let ev = PluginEvent::SpawnPane {
            command: "sh".into(),
            args: vec![],
            label: "x".into(),
        };
        let result = classify(&ev);
        assert_eq!(
            result,
            Some(HostAction::SpawnPane {
                command: "sh".into(),
                args: vec![],
                label: "x".into(),
            })
        );
    }

    #[test]
    fn classify_message_returns_none() {
        let ev = PluginEvent::Message {
            channel: "general".into(),
            sender: "bob".into(),
            text: "hello".into(),
            ts: "t".into(),
        };
        assert_eq!(classify(&ev), None);
    }

    #[test]
    fn classify_send_pane_returns_host_action() {
        let ev = PluginEvent::SendPane {
            label: "a".into(),
            text: "hi".into(),
        };
        assert_eq!(
            classify(&ev),
            Some(HostAction::SendPane {
                label: "a".into(),
                text: "hi".into(),
            })
        );
    }
}
