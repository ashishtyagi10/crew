//! Spawning chat/agent panes and resolving the bundled plugin command paths.
use crate::app::{CrewApp, FALLBACK_SIZE};
use crate::chat::ChatPane;
use crate::pane::{Pane, PaneContent};
use crate::spawn::PLACEHOLDER_RECT;
use crew_plugin::{Plugin, PluginCommand};

impl CrewApp {
    /// Spawn a new chat pane backed by the plugin at `cmd`.
    pub fn spawn_chat_pane(&mut self, cmd: &str) {
        self.spawn_plugin_pane(cmd, &[], None, None);
    }

    /// Spawn the `/smith` (alias `/crew`) pane: a chat pane backed by the
    /// multi-agent broker. The broker is `crew` itself re-exec'd with
    /// `--broker-plugin`, so it works wherever Crew is installed without a
    /// separate plugin binary. Named "crew" so its title bar distinguishes it
    /// from chat panes.
    ///
    /// **Guardrail:** `/smith` drives a heavyweight multi-agent broker
    /// subprocess (LLM agents, real spend), and the pane's `"crew"` label is its
    /// routing identity — session restore snapshots it by that name and host
    /// actions address it by it. A second broker would double the running cost
    /// and make label routing ambiguous, so if a crew pane is already open we
    /// focus it (restoring it if minimized, via `reconcile_grid`) instead of
    /// spawning a duplicate.
    pub(crate) fn spawn_crew_pane(&mut self) {
        if let Some(idx) = self
            .panes
            .iter()
            .position(|p| p.label.as_deref() == Some("crew"))
        {
            self.focused = idx;
            self.input.focused = false;
            self.set_status("crew pane already open — focusing it");
            return;
        }
        let cmd = Self::crew_broker_cmd();
        // label "crew" is the pane's routing identity — session restore
        // snapshots it by this, and host actions could address it.
        self.spawn_plugin_pane(
            &cmd,
            &["--broker-plugin".to_string()],
            Some("crew".to_string()),
            Some("crew".to_string()),
        );
    }

    /// Shared spawn path for plugin-backed panes (chat and crew). `name` sets
    /// the pane's title-bar label when present; `label` is the routing
    /// identity (host actions, session restore). On failure a status flash
    /// tells the user, rather than silently opening nothing.
    fn spawn_plugin_pane(
        &mut self,
        cmd: &str,
        args: &[String],
        name: Option<String>,
        label: Option<String>,
    ) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        match Plugin::spawn(cmd, args) {
            Ok(mut plugin) => {
                if let Err(e) = plugin.send(&PluginCommand::Hello { v: 1 }) {
                    eprintln!("spawn_plugin_pane: plugin hello error: {e}");
                }
                let chat = ChatPane::new(plugin, String::new());
                self.panes.push(Pane {
                    content: PaneContent::Chat(chat),
                    grid,
                    rect: PLACEHOLDER_RECT,
                    label,
                    name,
                    dir: None,
                    activity: false,
                    bell: false,
                    hidden: false,
                    attention: None,
                });
                self.focus_new_pane();
            }
            Err(e) => {
                eprintln!("spawn_plugin_pane failed: {e:#}");
                self.set_status(format!("could not start pane: {e}"));
            }
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

    /// Resolve the `/crew` multi-agent broker command. Defaults to **this**
    /// binary (`crew`), which the pane runs with `--broker-plugin` — so `/crew`
    /// works wherever Crew is installed, with no separate binary to ship. Set
    /// `CREW_BROKER_PLUGIN` to use a standalone `crew-broker-plugin` instead.
    pub(crate) fn crew_broker_cmd() -> String {
        std::env::var("CREW_BROKER_PLUGIN").unwrap_or_else(|_| {
            std::env::current_exe()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| "crew".to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::app::CrewApp;
    use crate::farpane::FarPane;
    use crate::layout::Rect;
    use crate::pane::{Pane, PaneContent};
    use crew_term::GridSize;

    /// A stand-in pane carrying `label` — enough to exercise the `/smith`
    /// single-instance guardrail without a real broker subprocess.
    fn labeled_pane(label: Option<&str>) -> Pane {
        Pane {
            content: PaneContent::Far(FarPane::new(std::env::temp_dir())),
            grid: GridSize { cols: 80, rows: 24 },
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            label: label.map(str::to_string),
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
        }
    }

    #[test]
    fn smith_focuses_the_existing_crew_pane_instead_of_spawning_a_second_broker() {
        let mut app = CrewApp::default();
        // A couple of unrelated panes, then the crew pane at index 2.
        app.panes.push(labeled_pane(None));
        app.panes.push(labeled_pane(None));
        app.panes.push(labeled_pane(Some("crew")));
        app.focused = 0;
        app.input.focused = true;

        app.spawn_crew_pane();

        assert_eq!(app.panes.len(), 3, "no duplicate broker pane was spawned");
        assert_eq!(app.focused, 2, "focus moved to the existing crew pane");
        assert!(!app.input.focused, "focus left the input bar for the pane");
    }

    #[test]
    fn a_minimized_crew_pane_is_the_guardrail_target_too() {
        // The guard matches by label regardless of hidden state; reconcile_grid
        // restores it on the next render because focus left the input bar.
        let mut app = CrewApp::default();
        app.panes.push(labeled_pane(Some("crew")));
        app.panes[0].hidden = true;
        app.focused = 0;
        app.input.focused = true;

        app.spawn_crew_pane();

        assert_eq!(app.panes.len(), 1, "still no second broker");
        assert_eq!(app.focused, 0);
        assert!(
            !app.input.focused,
            "focus off the input bar lets reconcile_grid restore the pane"
        );
    }
}
