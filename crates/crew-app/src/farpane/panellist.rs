//! One directory panel of the `/far` pane: its rounded frame with the path
//! tab on the top rule, and the listing with the cursor bar. Split from
//! [`super`] (the pane's layout) when the focused/blurred styling grew it past
//! the line cap.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, ListState, StatefulWidget, Widget};

use super::super::Panel;
use super::{dir_color, fmt_size, legend, rowfit};
use crate::palette::accent_color;

/// Render one directory panel: a rounded box (path as legend) with the listing.
pub(super) fn panel(buf: &mut Buffer, area: Rect, panel: &Panel, active: bool, focused: bool) {
    let t = crew_theme::theme();
    let (fill, on_fill) = match focused {
        true => (
            accent_color(),
            Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2),
        ),
        false => (rgb(crew_theme::readable::selection_bg(&t)), rgb(t.ink)),
    };
    let dim_col = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let text_col = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let edge = if active && focused {
        accent_color()
    } else {
        dim_col
    };
    // The active panel's legend is a FILLED accent tab (the F-key bar's pill
    // language) — the accent border alone was too subtle to tell which side
    // keys act on (user feedback, v0.6.23). Inactive stays plain dim text.
    let legend_style = if active {
        Style::new()
            .fg(on_fill)
            .bg(fill)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(dim_col)
    };
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(edge))
        .title(Span::styled(
            legend(
                &panel.loc.display(),
                panel.entries.len(),
                panel.entries.iter().map(|e| e.size).sum::<u64>(),
                area.width,
            ),
            legend_style,
        ));
    let inner = block.inner(area);
    block.render(area, buf);
    let h = inner.height.max(1) as usize;
    // Scroll so the cursor stays visible (bottom-anchored once it passes `h`).
    let start = panel.sel.saturating_sub(h.saturating_sub(1)).min(panel.sel);
    // A remote listing in flight (and nothing to show yet): one dim row
    // instead of an empty panel, so the pane doesn't look inert while the
    // `rclone lsjson` worker (see `remote.rs`) is still running.
    if panel.loading && panel.entries.is_empty() {
        let items = vec![ListItem::new(Line::from(Span::styled(
            "\u{27f3} listing\u{2026}",
            Style::new().fg(dim_col),
        )))];
        let mut state = ListState::default();
        state.select(Some(0));
        StatefulWidget::render(List::new(items), inner, buf, &mut state);
        return;
    }
    let items: Vec<ListItem> = panel
        .entries
        .iter()
        .skip(start)
        .take(h)
        .map(|e| {
            let width = inner.width as usize;
            let glyph = super::super::icons::icon(e);
            let (name, fg) = if e.is_dir {
                (format!("{glyph} {}/", e.name), dir_color())
            } else {
                (format!("{glyph} {}", e.name), text_col)
            };
            let size = if e.is_dir {
                String::new()
            } else {
                fmt_size(e.size)
            };
            let (name, pad) = rowfit::fit(name, &size, width);
            let mut spans = vec![Span::styled(name, Style::new().fg(fg))];
            if !size.is_empty() {
                spans.push(Span::styled(
                    format!("{}{size}", " ".repeat(pad)),
                    Style::new().fg(dim_col),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();
    // Only the ACTIVE panel gets a filled cursor bar — with a fill on both
    // sides it was ambiguous which panel keys would act on (the inactive
    // side's bar often sits on `../` and reads as "selected"). The inactive
    // panel remembers its place with a bold row instead of a bar.
    let hl = if active {
        Style::new().fg(on_fill).bg(fill)
    } else {
        Style::new().add_modifier(Modifier::BOLD)
    };
    let mut state = ListState::default();
    state.select(Some(panel.sel - start));
    StatefulWidget::render(List::new(items).highlight_style(hl), inner, buf, &mut state);
}

fn rgb((r, g, b): (u8, u8, u8)) -> Color {
    Color::Rgb(r, g, b)
}
