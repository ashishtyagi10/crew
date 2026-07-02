//! The type-to-search font-family dropdown: a rounded popup anchored below
//! the family row. The list marks the family that is CURRENTLY in the draft
//! with a leading `✓` (bold), separate from the `›` selection cursor, so you
//! always see which font is active while browsing candidates.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{
    Block, BorderType, Clear, List, ListItem, ListState, StatefulWidget, Widget,
};

use super::{SettingsPane, DEFAULT_FAMILY_LABEL};
use crate::palette::accent_color;

pub(crate) fn dropdown(buf: &mut Buffer, p: &SettingsPane, anchor: Rect) {
    let names = p.filtered();
    let want = names.len() as u16 + 2;
    let y0 = anchor.y + anchor.height; // just below the family row
    let max = buf.area.height.saturating_sub(y0);
    if max < 3 {
        return;
    }
    let height = want.clamp(3, max);
    let area = Rect::new(anchor.x, y0, anchor.width, height);
    Clear.render(area, buf);
    let current = p.draft.font_family.clone().unwrap_or_default();
    let items: Vec<ListItem> = names
        .into_iter()
        .map(|n| {
            let active = n == current || (current.is_empty() && n == DEFAULT_FAMILY_LABEL);
            if active {
                ListItem::new(format!("\u{2713} {n}"))
                    .style(Style::new().add_modifier(Modifier::BOLD))
            } else {
                ListItem::new(format!("  {n}"))
            }
        })
        .collect();
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(accent_color()))
        .title(Span::styled(" fonts ", Style::new().fg(accent_color())));
    let t = crew_theme::theme();
    let page_col = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::new().fg(page_col).bg(accent_color()))
        .highlight_symbol("\u{203a} ");
    let mut state = ListState::default();
    state.select(Some(p.family_sel));
    StatefulWidget::render(list, area, buf, &mut state);
}

#[cfg(test)]
mod tests {
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
}
