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
    let (larea, rarea) = split_panels(split[0]);
    panel(&mut buf, larea, &p.left, p.active == Side::Left);
    panel(&mut buf, rarea, &p.right, p.active == Side::Right);
    merge_divider(&mut buf, split[0], rarea.x);
    // Scroll thumbs paint last: the left panel's border is the shared middle
    // column, which the right panel's block render and merge_divider both
    // overwrite — so a thumb drawn inside panel() would be lost.
    scroll_thumb(&mut buf, larea, &p.left, p.active == Side::Left);
    scroll_thumb(&mut buf, rarea, &p.right, p.active == Side::Right);
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

/// Halve `area` with a one-column overlap, so the panels share their middle
/// border instead of drawing `││` (which reads as a wide gap on screen).
fn split_panels(area: Rect) -> (Rect, Rect) {
    let lw = area.width / 2 + 1;
    (
        Rect::new(area.x, area.y, lw, area.height),
        Rect::new(area.x + lw - 1, area.y, area.width - lw + 1, area.height),
    )
}

/// Join the shared border column into the panel frames: `┬` at the top, `┴`
/// at the bottom, accent-coloured — the divider always touches the active
/// panel, whichever side it is.
fn merge_divider(buf: &mut Buffer, area: Rect, x: u16) {
    for y in area.y..area.y + area.height {
        let sym = if y == area.y {
            "\u{252c}" // ┬
        } else if y == area.y + area.height - 1 {
            "\u{2534}" // ┴
        } else {
            "\u{2502}" // │
        };
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_symbol(sym);
            cell.set_fg(accent_color());
        }
    }
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
            legend(&panel.cwd, panel.entries.len(), area.width),
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
            let width = inner.width as usize;
            let glyph = super::icons::icon(e);
            let (mut name, fg) = if e.is_dir {
                (format!("{glyph} {}/", e.name), DIR)
            } else {
                (format!("{glyph} {}", e.name), text_col)
            };
            let size = if e.is_dir {
                String::new()
            } else {
                fmt_size(e.size)
            };
            if !size.is_empty() && name.chars().count() + size.chars().count() >= width {
                // Keep the size intact; truncate the name with an ellipsis
                // (the legend truncates the same way, from the other end).
                let keep = width.saturating_sub(size.chars().count() + 2);
                name = name.chars().take(keep).chain(['\u{2026}']).collect();
            }
            let pad = width.saturating_sub(name.chars().count() + size.chars().count());
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
    let hl = if active {
        Style::new().fg(page_col).bg(accent_color())
    } else {
        Style::new().fg(page_col).bg(dim_col)
    };
    let mut state = ListState::default();
    state.select(Some(panel.sel - start));
    StatefulWidget::render(List::new(items).highlight_style(hl), inner, buf, &mut state);
}

/// Paint the proportional scroll thumb over `panel`'s right border while its
/// listing overflows. Called from `render` AFTER both panels and the divider
/// are drawn, since the left panel's border is the shared middle column.
fn scroll_thumb(buf: &mut Buffer, area: Rect, panel: &Panel, active: bool) {
    let inner_h = area.height.saturating_sub(2) as usize; // minus top/bottom border
    let start = panel
        .sel
        .saturating_sub(inner_h.saturating_sub(1))
        .min(panel.sel);
    let Some((top, len)) = crate::chatscroll::thumb(panel.entries.len(), inner_h, start) else {
        return;
    };
    let edge = if active {
        accent_color()
    } else {
        let t = crew_theme::theme();
        Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2)
    };
    let x = area.x + area.width - 1;
    for i in 0..len {
        if let Some(cell) = buf.cell_mut((x, area.y + 1 + (top + i) as u16)) {
            cell.set_symbol("\u{2588}"); // █
            cell.set_fg(edge);
        }
    }
}

/// `bytes` in compact Far-style units: `427 B`, `1.2K`, `34M`, `2.1G` — one
/// decimal below 10, none above, binary (1024) steps.
fn fmt_size(bytes: u64) -> String {
    const UNITS: [char; 4] = ['K', 'M', 'G', 'T'];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut v = bytes as f64 / 1024.0;
    let mut i = 0;
    while v >= 1024.0 && i + 1 < UNITS.len() {
        v /= 1024.0;
        i += 1;
    }
    if v < 10.0 {
        format!("{v:.1}{}", UNITS[i])
    } else {
        format!("{v:.0}{}", UNITS[i])
    }
}

/// `" /path · N "` — `N` is the panel's entry count. The count suffix stays
/// intact whenever there's room for it at all; the path truncates from the
/// left (keeping the tail) to fit `width`, same as before the count was added.
fn legend(cwd: &std::path::Path, count: usize, width: u16) -> String {
    let full = cwd.to_string_lossy();
    let suffix = format!(" \u{00b7} {count} ");
    let max = (width as usize).saturating_sub(1 + suffix.chars().count());
    if full.chars().count() <= max || max == 0 {
        return format!(" {full}{suffix}");
    }
    let tail: String = full
        .chars()
        .rev()
        .take(max.saturating_sub(1))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!(" …{tail}{suffix}")
}

/// The Far-style function-key bar across the bottom row: the key number in
/// accent, a gap, then the action label on a solid accent pill. The pill's
/// padding is half-block glyphs (`▐label▌`), not spaces — `to_cells` drops
/// blank cells, so a bg-only space would never reach the GPU.
fn function_bar(buf: &mut Buffer, area: Rect) {
    let t = crew_theme::theme();
    let bar_bg = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let cap = Style::new().fg(accent_color());
    let mut spans = Vec::new();
    for (k, label) in FKEYS {
        spans.push(Span::styled(format!("F{k} "), cap));
        spans.push(Span::styled("\u{2590}", cap)); // ▐ left pill edge
        spans.push(Span::styled(
            label,
            Style::new().fg(bar_bg).bg(accent_color()),
        ));
        spans.push(Span::styled("\u{258c}", cap)); // ▌ right pill edge
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
