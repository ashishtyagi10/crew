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
    dropdown(&mut buf, &p, Rect::new(0, 0, 40, 1), &[]);
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

/// A card the list cuts through is blanked below it, not left as a stray
/// bottom border; a card it does not reach is untouched.
#[test]
fn a_card_the_list_cuts_through_is_blanked_below_it() {
    let mut p = SettingsPane::new(CrewConfig::default(), vec!["Lilex".into(), "Menlo".into()]);
    p.family_open = true;
    p.family_query.clear();
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 16));
    // The list is 3 names + 2 borders = rows 1..6; the cut card spans 4..7,
    // the untouched one 9..12.
    let (cut, below) = (Rect::new(2, 4, 20, 4), Rect::new(2, 9, 20, 3));
    for r in [cut, below] {
        ratatui::widgets::Block::bordered().render(r, &mut buf);
    }
    dropdown(&mut buf, &p, Rect::new(0, 0, 40, 1), &[cut, below]);
    assert!(
        !cell_text(&buf, 6).contains('\u{2518}'),
        "{:?}",
        cell_text(&buf, 6)
    );
    assert!(
        cell_text(&buf, 6).trim().is_empty(),
        "{:?}",
        cell_text(&buf, 6)
    );
    assert!(
        cell_text(&buf, 11).contains('\u{2518}'),
        "the far card keeps its frame"
    );
}
