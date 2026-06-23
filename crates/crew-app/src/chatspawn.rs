//! Spawning chat/agent panes and resolving the bundled plugin command paths.
use crate::app::{CrewApp, FALLBACK_SIZE};
use crate::chat::ChatPane;
use crate::pane::{Pane, PaneContent};
use crate::spawn::PLACEHOLDER_RECT;
use crew_plugin::{Plugin, PluginCommand};

impl CrewApp {
    /// Spawn a new chat pane backed by the plugin at `cmd`.
    pub fn spawn_chat_pane(&mut self, cmd: &str) {
        self.spawn_plugin_pane(cmd, None);
    }

    /// Spawn the `/crew` pane: a chat pane backed by the multi-agent broker
    /// plugin, named "crew" so its title bar distinguishes it from chat panes.
    pub(crate) fn spawn_crew_pane(&mut self) {
        let cmd = Self::crew_broker_cmd();
        self.spawn_plugin_pane(&cmd, Some("crew".to_string()));
    }

    /// Shared spawn path for plugin-backed panes (chat and crew). `name` sets
    /// the pane's title-bar label when present.
    fn spawn_plugin_pane(&mut self, cmd: &str, name: Option<String>) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        match Plugin::spawn(cmd, &[]) {
            Ok(mut plugin) => {
                if let Err(e) = plugin.send(&PluginCommand::Hello { v: 1 }) {
                    eprintln!("spawn_plugin_pane: plugin hello error: {e}");
                }
                let chat = ChatPane::new(plugin, String::new());
                self.panes.push(Pane {
                    content: PaneContent::Chat(chat),
                    grid,
                    rect: PLACEHOLDER_RECT,
                    label: None,
                    name,
                    activity: false,
                    bell: false,
                });
                self.focus_new_pane();
            }
            Err(e) => eprintln!("spawn_plugin_pane failed: {e:#}"),
        }
    }

    /// Resolve the echo plugin command path.
    pub(crate) fn echo_plugin_cmd() -> String {
        std::env::var("CREW_CHAT_PLUGIN").unwrap_or_else(|_| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("crew-echo-plugin")))
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| "crew-echo-plugin".to_string())
        })
    }

    /// Resolve the orchestrator plugin command path.
    pub(crate) fn orchestrator_plugin_cmd() -> String {
        std::env::var("CREW_ORCHESTRATOR_PLUGIN").unwrap_or_else(|_| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("crew-orchestrator-plugin")))
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| "crew-orchestrator-plugin".to_string())
        })
    }

    /// Resolve the `/crew` multi-agent broker plugin command path. Looks beside
    /// the running binary (where Cargo places sibling bins), overridable via
    /// `CREW_BROKER_PLUGIN`.
    pub(crate) fn crew_broker_cmd() -> String {
        std::env::var("CREW_BROKER_PLUGIN").unwrap_or_else(|_| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("crew-broker-plugin")))
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| "crew-broker-plugin".to_string())
        })
    }
}
