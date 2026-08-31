use super::*;
use crate::config::CrewConfig;

fn cell_text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
        .collect()
}

#[test]
fn dropdown_marks_the_draft_family_with_a_check() {
    let cfg = CrewConfig {
        font_family: Some("JetBrainsMono Nerd Font".into()),
        ..CrewConfig::default()
    };
    let mut p = SettingsPane::new(cfg, vec!["JetBrainsMono Nerd Font".into(), "Menlo".into()]);
    p.family_open = true;
    p.family_query.clear(); // empty query → the full list shows
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 12));
    dropdown(&mut buf, &p, Rect::new(0, 0, 40, 1));
    let all: String = (0..12).map(|y| cell_text(&buf, y) + "\n").collect();
    assert!(
        all.contains("\u{2713} JetBrainsMono Nerd Font"),
        "active family gets the check: {all}"
    );
    assert!(
        all.contains("  Menlo"),
        "others align under the marker: {all}"
    );
}
