use super::*;
use crate::config::CrewConfig;
use crate::layout::Rect;
use crate::pane::{Pane, PaneContent};
use crate::settingspane::SettingsPane;
use crew_term::GridSize;

/// A cheap label-only pane (Settings content) — `resolve` only reads label.
fn labeled(label: Option<&str>) -> Pane {
    Pane {
        glide: crate::glide::Glide::default(),
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
        born_ms: crate::anim::now_ms(),
    }
}

#[test]
fn split_instance_separates_pane_from_instance() {
    assert_eq!(split_instance("schema"), ("schema", None));
    assert_eq!(split_instance("schema@alpha"), ("schema", Some("alpha")));
    // Empty instance or empty pane → treated as a bare local address.
    assert_eq!(split_instance("schema@"), ("schema@", None));
    assert_eq!(split_instance("@alpha"), ("@alpha", None));
    // Splits on the LAST '@' (a label may contain one).
    assert_eq!(split_instance("a@b@host"), ("a@b", Some("host")));
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
fn wrap_includes_from_id_question_and_marker() {
    let w = wrap("builder", "q7", "which API?");
    assert!(w.contains("builder") && w.contains("which API?"));
    assert!(w.contains("CREW-ANS-q7:"), "answer marker present: {w}");
}

#[test]
fn scan_reads_a_line_starting_with_the_marker() {
    assert_eq!(
        scan_answer("noise\nCREW-ANS-q7: v2\ntail", "q7"),
        Some("v2".into())
    );
    // Indented answer line still matches (leading whitespace trimmed).
    assert_eq!(scan_answer("  CREW-ANS-q7:  v2 ", "q7"), Some("v2".into()));
}

#[test]
fn scan_ignores_the_marker_mid_line_and_wrong_ids() {
    // The echoed instruction mentions the marker mid-sentence → not a match.
    assert_eq!(
        scan_answer("print a line beginning with CREW-ANS-q7: then...", "q7"),
        None
    );
    assert_eq!(scan_answer("CREW-ANS-q9: other", "q7"), None);
    assert_eq!(scan_answer("no marker here", "q7"), None);
}
