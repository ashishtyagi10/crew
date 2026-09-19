//! One panel row as a ratatui `ListItem` — the drawing half of `/keys`,
//! split from [`super`] (child module) when the panel learned to say which
//! section belongs to the pane you are in and the file crossed its cap.
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::ListItem;

use crate::helplayout::Row;
use crate::palette::accent_color;

pub(super) fn items(rows: &[Row], col: usize, inner_w: usize) -> Vec<ListItem<'static>> {
    let t = crew_theme::theme();
    let text_col = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let dim_col = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let rule_col = Color::Rgb(t.border_normal.0, t.border_normal.1, t.border_normal.2);
    rows.iter()
        .map(|row| match row {
            Row::Spacer => ListItem::new(Line::from("")),
            // A heading used to be one more dim row at the same indent as the
            // keys — nothing about it said "new section". It now carries a
            // rule to the panel's edge, the same way every card in crew
            // states a boundary.
            Row::Head(h, mine) => {
                // The section for the pane you are in says so, since the
                // panel opened on it and a reader who scrolls away needs to
                // be able to find it again.
                let h = &match mine {
                    true => format!("{h} \u{b7} this pane"),
                    false => (*h).to_string(),
                };
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
        .collect()
}
