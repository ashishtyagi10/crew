//! Builds the `crew panes` roster from live pane state — the directory an
//! asking agent reads to discover and choose a target. Every pane gets a
//! stable-within-session `p{i}` id plus its friendly label, kind, foreground
//! agent, dir, and busy state.
use crate::ipc_types::PaneCard;
use crate::pane::{Pane, PaneContent};
use crate::procname::ProcNames;

/// One roster card for pane `i`.
pub(crate) fn card_for(i: usize, p: &Pane, procnames: &ProcNames) -> PaneCard {
    let (kind, running, busy) = match &p.content {
        PaneContent::Terminal(t) => {
            let cmd = t.pty.foreground_pid().and_then(|pid| procnames.name(pid));
            let busy = cmd.is_some(); // a foreground agent (claude/codex/…) is running
            ("terminal", cmd, busy)
        }
        PaneContent::Chat(c) => ("swarm", None, c.is_busy()),
        PaneContent::Swarm(_) => ("swarm", None, false),
        PaneContent::Far(_) => ("far", None, false),
        _ => ("other", None, false),
    };
    PaneCard {
        id: format!("p{i}"),
        label: p.name.clone().or_else(|| p.label.clone()),
        kind: kind.to_string(),
        running,
        dir: p
            .dir
            .as_ref()
            .and_then(|d| d.file_name())
            .map(|n| n.to_string_lossy().into_owned()),
        busy,
    }
}

/// The full roster.
pub(crate) fn roster(panes: &[Pane], procnames: &ProcNames) -> Vec<PaneCard> {
    panes
        .iter()
        .enumerate()
        .map(|(i, p)| card_for(i, p, procnames))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::ChatPane;
    use crate::config::CrewConfig;
    use crate::layout::Rect;
    use crate::settingspane::SettingsPane;
    use crew_term::GridSize;

    fn pane(content: PaneContent, label: Option<&str>) -> Pane {
        Pane {
            content,
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
    fn roster_reports_id_label_and_kind() {
        let plugin =
            crew_plugin::Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()])
                .unwrap();
        let panes = vec![
            pane(
                PaneContent::Settings(SettingsPane::new(CrewConfig::default(), vec![])),
                None,
            ),
            pane(
                PaneContent::Chat(ChatPane::new(plugin, "crew".into())),
                Some("crew"),
            ),
        ];
        let pn = ProcNames::default();
        let cards = roster(&panes, &pn);
        assert_eq!(cards[0].id, "p0");
        assert_eq!(cards[0].kind, "other");
        assert_eq!(cards[1].id, "p1");
        assert_eq!(cards[1].kind, "swarm");
        assert_eq!(cards[1].label.as_deref(), Some("crew"));
        assert!(!cards[1].busy, "idle chat pane is not busy");
    }
}
