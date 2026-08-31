use crate::{PluginCommand, PluginEvent};

pub fn plan(cmd: &PluginCommand) -> Vec<PluginEvent> {
    match cmd {
        PluginCommand::Hello { .. } => vec![PluginEvent::Ready {
            v: 1,
            provider: "orchestrator".into(),
            channels: vec!["plan".into()],
        }],
        PluginCommand::Send { text, .. } => vec![
            PluginEvent::Message {
                channel: "plan".into(),
                sender: "orchestrator".into(),
                text: format!("Plan: spawning 2 agents for: {text}"),
                ts: String::new(),
                meta: String::new(),
            },
            PluginEvent::SpawnPane {
                command: "sh".into(),
                args: vec!["-c".into(), format!("echo agent-A on: {text}; sleep 30")],
                label: "agent-A".into(),
            },
            PluginEvent::SpawnPane {
                command: "sh".into(),
                args: vec!["-c".into(), format!("echo agent-B on: {text}; sleep 30")],
                label: "agent-B".into(),
            },
        ],
        // The orchestrator plugin has no gate: nothing here asks, so nothing answers.
        PluginCommand::Subscribe { .. } | PluginCommand::Approve { .. } => vec![],
    }
}

#[cfg(test)]
#[path = "orchestrator_tests.rs"]
mod tests;
