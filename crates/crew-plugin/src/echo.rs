use crate::{PluginCommand, PluginEvent};

pub fn respond(cmd: &PluginCommand) -> Vec<PluginEvent> {
    match cmd {
        PluginCommand::Hello { .. } => vec![PluginEvent::Ready {
            v: 1,
            provider: "echo".into(),
            channels: vec!["general".into()],
        }],
        PluginCommand::Send { channel, text } => vec![PluginEvent::Message {
            channel: channel.clone(),
            sender: "echo".into(),
            text: text.clone(),
            ts: String::new(),
            meta: String::new(),
        }],
        // The echo plugin has no gate, so it never asks and never has to answer.
        PluginCommand::Subscribe { .. } | PluginCommand::Approve { .. } => vec![],
    }
}

#[cfg(test)]
#[path = "echo_tests.rs"]
mod tests;
