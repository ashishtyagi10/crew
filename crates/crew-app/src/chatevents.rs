//! Plugin-event classification for chat panes: which events are host actions
//! (pane spawns / sends) the app must perform, versus pane-local state changes.
use crew_plugin::PluginEvent;

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
    HivePlan {
        tasks: Vec<crew_hive::TaskSpec>,
    },
    Hive {
        event: crew_hive::HiveEvent,
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
        PluginEvent::HivePlan { tasks } => Some(HostAction::HivePlan {
            tasks: tasks.clone(),
        }),
        PluginEvent::Hive { event } => Some(HostAction::Hive {
            event: event.clone(),
        }),
        _ => None,
    }
}
