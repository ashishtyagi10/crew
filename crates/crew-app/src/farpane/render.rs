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

use crate::palette::accent_color;
/// Blue-cyan for directory entries (semantic file type indicator).
const DIR: Color = Color::Rgb(120, 200, 255);

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
    if cols < 16 || rows < 5 {
        return Vec::new();
    }
    let area = Rect::new(0, 0, cols, rows);
    let mut buf = Buffer::empty(area);
    // Panels, then the command line, then the function-key bar.
    let split = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area);
    let cols2 = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(split[0]);
    panel(&mut buf, cols2[0], &p.left, p.active == Side::Left);
    panel(&mut buf, cols2[1], &p.right, p.active == Side::Right);
    let running = p.running.as_ref().map(|(cmd, _)| cmd.as_str());
    command_bar(&mut buf, split[1], &p.active_cwd(), &p.cmdline, running);
    // The make-folder prompt takes over the function-key row while it's open.
    match &p.prompt {
        Some(prompt) => prompt_bar(&mut buf, split[2], prompt),
        None => function_bar(&mut buf, split[2]),
    }
    crate::tui::to_cells(&buf)
}

/// The Far command line: `<cwd> $ <typed>▏`, the directory dimmed and the typed
/// command in the ink colour with a cursor bar. While a command runs, a dimmed
/// `⟳ <cmd>` note follows the prompt. Truncated from the left to fit.
fn command_bar(
    buf: &mut Buffer,
    area: Rect,
    cwd: &std::path::Path,
    cmdline: &str,
    running: Option<&str>,
) {
    let t = crew_theme::theme();
    let bg = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let dim = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let ink = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let folder = cwd
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd.to_string_lossy().into_owned());
    let mut spans = vec![
        Span::styled(format!("{folder} "), Style::new().fg(dim).bg(bg)),
        Span::styled("$ ", Style::new().fg(accent_color()).bg(bg)),
        Span::styled(format!("{cmdline}▏"), Style::new().fg(ink).bg(bg)),
    ];
    if let Some(cmd) = running {
        spans.push(Span::styled(
            format!("  \u{27f3} {cmd}"),
            Style::new().fg(dim).bg(bg),
        ));
    }
    Paragraph::new(Line::from(spans))
        .style(Style::new().bg(bg))
        .render(area, buf);
}

/// Render one directory panel: a rounded box (path as legend) with the listing.
fn panel(buf: &mut Buffer, area: Rect, panel: &Panel, active: bool) {
    let t = crew_theme::theme();
    let dim_col = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let text_col = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let page_col = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let edge = if active { accent_color() } else { dim_col };
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
            let fg = if e.is_dir { DIR } else { text_col };
            ListItem::new(Line::styled(label, Style::new().fg(fg)))
        })
        .collect();
    let hl = if active {
        Style::new().fg(page_col).bg(accent_color())
    } else {
        Style::new().fg(page_col).bg(dim_col)
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
    let t = crew_theme::theme();
    let bar_bg = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let bar_fg = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let mut spans = Vec::new();
    for (k, label) in FKEYS {
        spans.push(Span::styled(
            format!("F{k}"),
            Style::new().fg(accent_color()).bg(bar_bg),
        ));
        spans.push(Span::styled(
            format!("{label} "),
            Style::new().fg(bar_fg).bg(bar_bg),
        ));
    }
    Paragraph::new(Line::from(spans))
        .style(Style::new().bg(bar_bg))
        .render(area, buf);
}

/// The bottom-row text prompt (F7 make-folder), replacing the function bar.
fn prompt_bar(buf: &mut Buffer, area: Rect, prompt: &super::Prompt) {
    let t = crew_theme::theme();
    let bar_bg = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let bar_fg = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let label = match prompt.kind {
        super::PromptKind::MkDir => "Create folder: ",
    };
    let line = format!("{label}{}▏", prompt.input);
    Paragraph::new(Line::from(Span::styled(
        line,
        Style::new().fg(bar_fg).bg(bar_bg),
    )))
    .style(Style::new().bg(bar_bg))
    .render(area, buf);
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
