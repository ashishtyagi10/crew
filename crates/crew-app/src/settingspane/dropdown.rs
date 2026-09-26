//! The type-to-search font-family dropdown: a rounded popup anchored below
//! the family row. The list marks the family that is CURRENTLY in the draft
//! with a leading `✓` (bold), separate from the `›` selection cursor, so you
//! always see which font is active while browsing candidates.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{
    Block, BorderType, Clear, List, ListItem, ListState, StatefulWidget, Widget,
};

use super::{SettingsPane, DEFAULT_FAMILY_LABEL};
use crate::palette::accent_color;

/// `cards` are the form's field rects in `buf`'s coordinates: any the list
/// cuts through has its remainder blanked ([`hide_cut`]).
pub(crate) fn dropdown(buf: &mut Buffer, p: &SettingsPane, anchor: Rect, cards: &[Rect]) {
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
    hide_cut(buf, area, cards);
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
    // No background bar — the cursor is the `›` and the bold, the same idiom
    // every list on the canvas uses (`cmdmenu`); a bar of accent under page-
    // coloured text was the one highlight in crew that shouted.
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::new().fg(accent_color()).add_modifier(Modifier::BOLD))
        .highlight_symbol("\u{203a} ");
    let mut state = ListState::default();
    state.select(Some(p.family_sel));
    StatefulWidget::render(list, area, buf, &mut state);
}

/// Blank what the list left of each card it cut through. A card whose top it
/// covers and whose bottom it does not showed that bottom as a stray `╰──╯`
/// under the list — two cards' worth of fragments beneath `fonts`, read as
/// broken frames rather than as something the list is covering.
fn hide_cut(buf: &mut Buffer, list: Rect, cards: &[Rect]) {
    for c in cards {
        let cut = (list.y..list.bottom()).contains(&c.y) && c.bottom() > list.bottom();
        if cut && c.x < list.right() && c.right() > list.x {
            let rest = Rect::new(c.x, list.bottom(), c.width, c.bottom() - list.bottom());
            Clear.render(rest.intersection(buf.area), buf);
        }
    }
}

#[cfg(test)]
#[path = "dropdown_tests.rs"]
mod tests;
