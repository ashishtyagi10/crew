//! Dual-pane file-manager rendering: two bordered directory panels side by side
//! (the active one accent-bordered, its cursor highlighted) over a Far-style
//! function-key bar. Built with ratatui and handed to the GPU as cells.
use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, List, ListItem, ListState, Paragraph, StatefulWidget, Widget,
};

use super::{FarPane, Panel, Side};

const ACCENT: Color = Color::Rgb(0, 255, 160);
const TEXT: Color = Color::Rgb(200, 200, 200);
const DIM: Color = Color::Rgb(120, 130, 140);
const DIR: Color = Color::Rgb(120, 200, 255);
const BAR_BG: Color = Color::Rgb(0, 40, 80);
const BAR_FG: Color = Color::Rgb(230, 230, 230);
const BLACK: Color = Color::Rgb(0, 0, 0);

/// Function-key labels shown along the bottom bar (classic Far layout).
const FKEYS: [(&str, &str); 8] = [
    ("1", "Help"),
    ("3", "View"),
    ("4", "Edit"),
    ("5", "Copy"),
    ("6", "RenMov"),
    ("7", "MkFold"),
    ("8", "Delete"),
    ("10", "Quit"),
];

pub(crate) fn render(p: &FarPane, cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 16 || rows < 4 {
        return Vec::new();
    }
    let area = Rect::new(0, 0, cols, rows);
    let mut buf = Buffer::empty(area);
    let split = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(area);
    let cols2 = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(split[0]);
    panel(&mut buf, cols2[0], &p.left, p.active == Side::Left);
    panel(&mut buf, cols2[1], &p.right, p.active == Side::Right);
    function_bar(&mut buf, split[1]);
    crate::tui::to_cells(&buf)
}

/// Render one directory panel: a rounded box (path as legend) with the listing.
fn panel(buf: &mut Buffer, area: Rect, panel: &Panel, active: bool) {
    let edge = if active { ACCENT } else { DIM };
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(edge))
        .title(Span::styled(
            legend(&panel.cwd, area.width),
            Style::new().fg(edge),
        ));
    let inner = block.inner(area);
    block.render(area, buf);
    let h = inner.height.max(1) as usize;
    // Scroll so the cursor stays visible (bottom-anchored once it passes `h`).
    let start = panel.sel.saturating_sub(h.saturating_sub(1)).min(panel.sel);
    let items: Vec<ListItem> = panel
        .entries
        .iter()
        .skip(start)
        .take(h)
        .map(|e| {
            let label = if e.is_dir {
                format!("{}/", e.name)
            } else {
                e.name.clone()
            };
            let fg = if e.is_dir { DIR } else { TEXT };
            ListItem::new(Line::styled(label, Style::new().fg(fg)))
        })
        .collect();
    let hl = if active {
        Style::new().fg(BLACK).bg(ACCENT)
    } else {
        Style::new().fg(BLACK).bg(DIM)
    };
    let mut state = ListState::default();
    state.select(Some(panel.sel - start));
    StatefulWidget::render(List::new(items).highlight_style(hl), inner, buf, &mut state);
}

/// `" /path "`, truncated from the left (keeping the tail) to fit `width`.
fn legend(cwd: &std::path::Path, width: u16) -> String {
    let full = cwd.to_string_lossy();
    let max = width.saturating_sub(4) as usize;
    if full.chars().count() <= max || max == 0 {
        return format!(" {full} ");
    }
    let tail: String = full
        .chars()
        .rev()
        .take(max.saturating_sub(1))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!(" …{tail} ")
}

/// The Far-style function-key bar across the bottom row.
fn function_bar(buf: &mut Buffer, area: Rect) {
    let mut spans = Vec::new();
    for (k, label) in FKEYS {
        spans.push(Span::styled(
            format!("F{k}"),
            Style::new().fg(ACCENT).bg(BAR_BG),
        ));
        spans.push(Span::styled(
            format!("{label} "),
            Style::new().fg(BAR_FG).bg(BAR_BG),
        ));
    }
    Paragraph::new(Line::from(spans))
        .style(Style::new().bg(BAR_BG))
        .render(area, buf);
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
