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
        PluginCommand::Subscribe { .. } => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_returns_ready_with_orchestrator_provider() {
        let events = plan(&PluginCommand::Hello { v: 1 });
        assert_eq!(events.len(), 1);
        match &events[0] {
            PluginEvent::Ready {
                v,
                provider,
                channels,
            } => {
                assert_eq!(*v, 1);
                assert_eq!(provider, "orchestrator");
                assert_eq!(channels, &vec!["plan".to_string()]);
            }
            _ => panic!("expected Ready event"),
        }
    }

    #[test]
    fn send_returns_message_and_two_spawn_panes() {
        let events = plan(&PluginCommand::Send {
            channel: "plan".into(),
            text: "build X".into(),
        });
        assert_eq!(events.len(), 3);

        // [0] is a Message whose text contains "build X"
        match &events[0] {
            PluginEvent::Message {
                channel,
                sender,
                text,
                ts,
            } => {
                assert_eq!(channel, "plan");
                assert_eq!(sender, "orchestrator");
                assert!(text.contains("build X"), "text: {text}");
                assert_eq!(ts, "");
            }
            _ => panic!("expected Message at index 0"),
        }

        // [1] is SpawnPane with label "agent-A" and args containing "build X"
        match &events[1] {
            PluginEvent::SpawnPane {
                command,
                args,
                label,
            } => {
                assert_eq!(label, "agent-A");
                assert_eq!(command, "sh");
                assert!(args.join(" ").contains("build X"), "args: {args:?}");
            }
            _ => panic!("expected SpawnPane at index 1"),
        }

        // [2] is SpawnPane with label "agent-B" and args containing "build X"
        match &events[2] {
            PluginEvent::SpawnPane {
                command,
                args,
                label,
            } => {
                assert_eq!(label, "agent-B");
                assert_eq!(command, "sh");
                assert!(args.join(" ").contains("build X"), "args: {args:?}");
            }
            _ => panic!("expected SpawnPane at index 2"),
        }
    }

    #[test]
    fn subscribe_returns_empty() {
        let events = plan(&PluginCommand::Subscribe {
            channel: "plan".into(),
        });
        assert!(events.is_empty());
    }
}
