//! The tool lane of one relay hop (`HopStream::on_tool`): each hive event
//! an agent's OWN runtime reports — Claude Code calling `Read`, and the
//! result — is forwarded as the `Hive` event the pane's tool block draws,
//! followed by the `Activity` the swarm path pairs it with (`swarmmsg`):
//! `tool <label>` from `"hive"` while the call runs, bare `tool` when it
//! returns. The agent id inside is minted from the name
//! (`AgentId::minted`), which the app mints the same way from the roster,
//! so the block is named `claude` from its first line.
use crew_hive::HiveEvent;

use crate::PluginEvent;

pub(crate) fn hop_tooler(
    tick_emit: std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync>,
    agent: String,
) -> std::sync::Arc<dyn Fn(HiveEvent) + Send + Sync> {
    std::sync::Arc::new(move |event: HiveEvent| {
        let activity = match &event {
            HiveEvent::ToolCall { label, .. } => Some(PluginEvent::Activity {
                agent: agent.clone(),
                state: format!("tool {label}"),
                from: "hive".into(),
            }),
            HiveEvent::ToolResult { .. } => Some(PluginEvent::Activity {
                agent: agent.clone(),
                state: "tool".into(),
                from: String::new(),
            }),
            _ => None,
        };
        tick_emit(PluginEvent::Hive { event });
        if let Some(a) = activity {
            tick_emit(a);
        }
    })
}

#[cfg(test)]
#[path = "hoptool_tests.rs"]
mod tests;
