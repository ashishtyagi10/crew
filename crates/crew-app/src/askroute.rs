//! Address resolution and the cooperative-sentinel wrap/scan for inter-pane
//! `ask`. Pure string/lookup logic — the socket, injection, and liveness
//! wait live elsewhere (ipc.rs / askpump / askwait.rs).
use crate::pane::Pane;

/// Resolve an address to a pane index: an exact `/name`-or-label match first,
/// else the `p{index}` fallback form (every pane is addressable even unnamed).
pub(crate) fn resolve(panes: &[Pane], addr: &str) -> Option<usize> {
    if let Some(i) = panes
        .iter()
        .position(|p| p.name.as_deref() == Some(addr) || p.label.as_deref() == Some(addr))
    {
        return Some(i);
    }
    addr.strip_prefix('p')
        .and_then(|n| n.parse::<usize>().ok())
        .filter(|&i| i < panes.len())
}

/// The sentinel-wrapped question injected into the target pane's live session.
/// `id` namespaces the answer markers so concurrent asks don't collide.
pub(crate) fn wrap(from: &str, id: &str, question: &str) -> String {
    format!(
        "\n[\u{21d0} ask from \"{from}\" \u{00b7} {id}] {question}\n\
         Reply between <CREW-ANS {id}> and </CREW-ANS {id}>.\n"
    )
}

/// Extract the answer from `captured` output: the text between a
/// `<CREW-ANS id>` open and its matching `</CREW-ANS id>` close. `None` until
/// the close marker has arrived (a partial, un-closed answer isn't done).
pub(crate) fn scan_answer(captured: &str, id: &str) -> Option<String> {
    let open = format!("<CREW-ANS {id}>");
    let close = format!("</CREW-ANS {id}>");
    let start = captured.find(&open)? + open.len();
    let rest = &captured[start..];
    let end = rest.find(&close)?;
    Some(rest[..end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CrewConfig;
    use crate::layout::Rect;
    use crate::pane::{Pane, PaneContent};
    use crate::settingspane::SettingsPane;
    use crew_term::GridSize;

    /// A cheap label-only pane (Settings content) — `resolve` only reads label.
    fn labeled(label: Option<&str>) -> Pane {
        Pane {
            content: PaneContent::Settings(SettingsPane::new(CrewConfig::default(), vec![])),
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
    fn resolve_by_label_then_index() {
        let panes = vec![labeled(None), labeled(Some("schema"))];
        assert_eq!(resolve(&panes, "schema"), Some(1));
        assert_eq!(resolve(&panes, "p0"), Some(0));
        assert_eq!(resolve(&panes, "p9"), None, "out-of-range index");
        assert_eq!(resolve(&panes, "nope"), None);
    }

    #[test]
    fn wrap_includes_from_id_question_and_sentinels() {
        let w = wrap("builder", "q7", "which API?");
        assert!(w.contains("builder") && w.contains("q7") && w.contains("which API?"));
        assert!(w.contains("<CREW-ANS q7>") && w.contains("</CREW-ANS q7>"));
    }

    #[test]
    fn scan_extracts_between_markers_only_when_closed() {
        assert_eq!(
            scan_answer("noise <CREW-ANS q7>v2</CREW-ANS q7> tail", "q7"),
            Some("v2".into())
        );
        assert_eq!(scan_answer("<CREW-ANS q7>partial no close", "q7"), None);
        assert_eq!(scan_answer("<CREW-ANS q9>other</CREW-ANS q9>", "q7"), None);
    }
}
