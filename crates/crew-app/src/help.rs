//! Keyboard-shortcuts overlay (`/keys`): a centered rounded popup listing the
//! bindings, rendered with ratatui and dismissed by any key press.
use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, Widget};

use crate::palette::accent_color;

use crate::helplayout::{self, sections, Row, KEY_COL};
use crate::helptable::{
    BINDINGS, CHAT_BINDINGS, FAR_BINDINGS, SETTINGS_BINDINGS, TODO_BINDINGS, VIEW_BINDINGS,
};

/// Preferred overlay size in cells: both binding tables, a spacer and a
/// heading between them, plus borders/title/hint.
///
/// It used to include every slash command as well, which made 58 rows —
/// taller than a default window, so the commands were silently cut off at
/// the bottom. They are not here any more: the composer's palette has listed
/// them since v0.6.52, grouped and filterable, which is strictly better than
/// an unscrollable overflowing column. `/keys` is for keys.
///
/// The WIDTH is measured the same way, and was not: it was the constant 58,
/// which left 30 columns for a description, and eight rows had outgrown that.
/// ratatui clips a `Line` without a word of complaint, so `Esc` read "Discard
/// a pending plan · inter" — losing both the interrupt and the close — and
/// `@a+b` lost "in parallel", which is the entire point of the binding. The
/// same lesson as the footer in v0.6.57: a truncated instruction teaches the
/// half that fits, and nobody can see that the rest existed.
pub fn size() -> (u16, u16) {
    let rows = sections()
        .iter()
        .map(|(_, table)| table.len() + 2)
        .sum::<usize>()
        + BINDINGS.len()
        + 4;
    // The column the keys actually get (the widest key plus its gap), then
    // the longest description beside it. Asking for less than this is asking
    // for the panel to wrap, which it now does gracefully — but the size it
    // *prefers* is the one where nothing has to.
    let col = helplayout::widest_key() + 2;
    let widest = BINDINGS
        .iter()
        .chain(CHAT_BINDINGS)
        .chain(VIEW_BINDINGS)
        .chain(FAR_BINDINGS)
        .chain(TODO_BINDINGS)
        .chain(SETTINGS_BINDINGS)
        .map(|(_, d)| d.chars().count())
        .max()
        .unwrap_or(KEY_COL);
    ((col + widest + 2) as u16, rows as u16)
}

/// Rows of the list the overlay can show at `rows` cells tall (borders take
/// two). Zero when there is no room for a single row.
fn visible_rows(rows: u16) -> usize {
    usize::from(rows.saturating_sub(2))
}

/// The furthest the overlay can be scrolled: enough to bring the last row
/// into view, and not one row more — scrolling into blank space below a list
/// is the thing that makes an unfamiliar scroll feel broken.
pub fn max_scroll(rows: u16, cols: u16, needle: &str) -> usize {
    helplayout::rows(needle, cols)
        .len()
        .saturating_sub(visible_rows(rows))
}

/// Render the help overlay into a `cols × rows` grid, starting `scroll` rows
/// down the list.
///
/// It did not used to scroll, which made its height a hard budget: the list
/// had to fit a default window or ratatui cut the bottom off in silence, and
/// `the_overlay_fits_a_default_window` enforced that by failing whenever a
/// binding was added. Three times in one release the fix was to *merge two
/// rows* to make room — losing detail from bindings that had nothing to do
/// with the new one. A list that scrolls has no budget to spend.
pub fn help_cells(cols: u16, rows: u16, scroll: usize, needle: &str) -> Vec<CellView> {
    if cols < 12 || rows < 4 {
        return Vec::new();
    }
    let scroll = scroll.min(max_scroll(rows, cols, needle));
    let t = crew_theme::theme();
    let text_col = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let dim_col = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let rule_col = Color::Rgb(t.border_normal.0, t.border_normal.1, t.border_normal.2);
    let panel_col = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let mut buf = Buffer::empty(Rect::new(0, 0, cols, rows));
    let col = helplayout::key_col(cols);
    let inner_w = (cols as usize).saturating_sub(2);
    let all = helplayout::rows(needle, cols);
    let items: Vec<ListItem> = all[scroll.min(all.len())..]
        .iter()
        .map(|row| match row {
            Row::Spacer => ListItem::new(Line::from("")),
            // A heading used to be one more dim row at the same indent as the
            // keys — nothing about it said "new section". It now carries a
            // rule to the panel's edge, the same way every card in crew
            // states a boundary.
            Row::Head(h) => {
                let used = crate::chatwidth::str_w(h) + 1;
                let rule = "\u{2500}".repeat(inner_w.saturating_sub(used));
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{h} "), Style::new().fg(dim_col)),
                    Span::styled(rule, Style::new().fg(rule_col)),
                ]))
            }
            // Pad to the key column — and when a key is wider than it, give
            // it two spaces of its own rather than letting the description
            // run into it.
            Row::Bind(k, d) => {
                let w = crate::chatwidth::str_w(k);
                let pad = " ".repeat(col.saturating_sub(w).max(2));
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{k}{pad}"), Style::new().fg(accent_color())),
                    Span::styled(d.clone(), Style::new().fg(text_col)),
                ]))
            }
            Row::Cont(d) => ListItem::new(Line::from(vec![
                Span::raw(" ".repeat(col)),
                Span::styled(d.clone(), Style::new().fg(text_col)),
            ])),
            // A search that matches nothing must say so; an empty panel reads
            // as a rendering fault.
            Row::Note(n) => ListItem::new(Line::from(Span::styled(
                n.clone(),
                Style::new().fg(dim_col),
            ))),
        })
        .collect();
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(accent_color()))
        .style(Style::new().bg(panel_col))
        .title(Span::styled(
            match needle.is_empty() {
                true => format!(" keys \u{b7} crew v{} ", env!("CARGO_PKG_VERSION")),
                // What you typed, shown where the version was: a filter you
                // cannot see is a list that looks broken.
                false => format!(" keys \u{b7} {needle}\u{2588} "),
            },
            Style::new().fg(accent_color()),
        ));
    let inner = block.inner(buf.area);
    block.render(buf.area, &mut buf);
    List::new(items).render(inner, &mut buf);
    // Dismissal hint on the bottom border — and, while there is more list
    // than window, how to reach the rest of it. A scrollable thing that never
    // says so is one nobody scrolls.
    let more = scroll < max_scroll(rows, cols, needle);
    let hint = match (scroll > 0, more) {
        (_, true) => " \u{2191}\u{2193} for more \u{b7} type to filter \u{b7} esc ",
        (true, false) => " \u{2191} for the rest \u{b7} type to filter \u{b7} esc ",
        _ => " type to filter \u{b7} esc to close ",
    };
    let hint_col = cols.saturating_sub(hint.chars().count() as u16 + 2);
    for (i, ch) in hint.chars().enumerate() {
        let col = hint_col + i as u16;
        if let Some(cell) = buf.cell_mut((col, rows - 1)) {
            cell.set_char(ch).set_fg(dim_col);
        }
    }
    crate::tui::to_cells_opaque(&buf)
}

#[cfg(test)]
#[path = "help_tests.rs"]
mod tests;
