use super::*;
use crate::chat::ChatPane;
use crate::config::CrewConfig;
use crate::layout::Rect;
use crate::settingspane::SettingsPane;
use crew_term::GridSize;

fn pane(content: PaneContent, label: Option<&str>) -> Pane {
    Pane {
        glide: crate::glide::Glide::default(),
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
        born_ms: crate::anim::now_ms(),
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
