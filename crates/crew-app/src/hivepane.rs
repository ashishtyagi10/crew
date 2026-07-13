//! Companion swarm-graph pane for broker-side /crew swarms: HivePlan opens
//! (or refreshes) one pane labeled "hive"; Hive events feed its fleet.
use crate::app::{CrewApp, FALLBACK_SIZE};
use crate::pane::{Pane, PaneContent};
use crate::spawn::PLACEHOLDER_RECT;
use crate::swarmpane::SwarmPane;

/// The label identifying the single companion pane. Lookup is by label so
/// pane-index churn (close/reorder) can never orphan it.
const HIVE_LABEL: &str = "hive";

impl CrewApp {
    pub(crate) fn hive_pane_idx(&self) -> Option<usize> {
        self.panes.iter().position(|p| {
            p.label.as_deref() == Some(HIVE_LABEL) && matches!(p.content, PaneContent::Swarm(_))
        })
    }

    /// A swarm plan landed: open the companion pane, or reset an existing one
    /// to the new run's graph. Never steals focus — the chat pane is primary.
    pub(crate) fn hive_plan(&mut self, tasks: Vec<crew_hive::TaskSpec>) {
        let pane = match SwarmPane::for_remote(tasks) {
            Ok(p) => p,
            Err(e) => {
                self.set_status(format!("swarm plan invalid: {e}"));
                return;
            }
        };
        match self.hive_pane_idx() {
            Some(i) => {
                self.panes[i].content = PaneContent::Swarm(pane);
            }
            None => {
                self.panes.push(Pane {
                    content: PaneContent::Swarm(pane),
                    grid: FALLBACK_SIZE,
                    rect: PLACEHOLDER_RECT,
                    label: Some(HIVE_LABEL.to_string()),
                    name: None,
                    dir: None,
                    activity: false,
                    bell: false,
                    hidden: false,
                    attention: None,
                });
            }
        }
        self.redraw();
    }

    /// Forwarded telemetry for the companion pane; ignored when absent.
    pub(crate) fn hive_event(&mut self, event: &crew_hive::HiveEvent) {
        if let Some(i) = self.hive_pane_idx() {
            if let PaneContent::Swarm(s) = &mut self.panes[i].content {
                if s.apply_remote(event) {
                    self.redraw();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::CrewApp;
    use crew_hive::{AgentKind, ModelTier, TaskId, TaskSpec};

    fn plan() -> Vec<TaskSpec> {
        vec![TaskSpec {
            id: TaskId(0),
            title: "t0".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Cheap,
            deps: vec![],
            prompt: "p".into(),
        }]
    }

    #[test]
    fn hive_plan_opens_one_companion_pane_and_reuses_it() {
        let mut app = CrewApp::default();
        app.hive_plan(plan());
        assert_eq!(app.panes.len(), 1);
        assert_eq!(app.panes[0].label.as_deref(), Some("hive"));
        // A second run reuses the pane instead of stacking a new one.
        app.hive_plan(plan());
        assert_eq!(app.panes.len(), 1);
    }

    #[test]
    fn hive_event_without_pane_is_ignored() {
        let mut app = CrewApp::default();
        // Must not panic or create panes.
        app.hive_event(&crew_hive::HiveEvent::TaskStateChanged {
            task: TaskId(0),
            state: crew_hive::TaskState::Running,
        });
        assert!(app.panes.is_empty());
    }
}
