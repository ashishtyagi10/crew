//! Dual-pane file-manager rendering: two bordered directory panels side by side
//! (the active one accent-bordered, its cursor highlighted) over a Far-style
//! function-key bar. Built with ratatui and handed to the GPU as cells.
use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, List, ListItem, ListState, Paragraph, StatefulWidget, Widget,
};

use super::{FarPane, Panel, Side};

use crate::palette::accent_color;
/// Blue-cyan for directory entries (semantic file type indicator).
const DIR: Color = Color::Rgb(120, 200, 255);

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
    // A Tab-cycle already shows its candidate in `cmdline` directly; the
    // ghost suggestion would be confusing layered on top of it, so it's
    // suppressed while a cycle is active.
    let ghost = if p.complete.is_none() {
        p.history
            .ghost(&p.cmdline)
            .map(|full| full[p.cmdline.len()..].to_string())
    } else {
        None
    };
    // The `!` ask's live status: elapsed seconds while thinking (recomputed
    // fresh every frame from the stored `Instant` — nothing to tick), or
    // the accept/discard/edit hint once a suggestion has landed.
    let (ask_hint, suggested) = match &p.ask {
        Some(super::ask::AskState::Thinking { started, .. }) => (
            Some(format!("thinking\u{2026} {}s", started.elapsed().as_secs())),
            false,
        ),
        Some(super::ask::AskState::Suggested { .. }) => (
            Some("Enter run \u{b7} Esc discard \u{b7} keep typing to edit".to_string()),
            true,
        ),
        None => (None, false),
    };
    let running = p.running.as_ref().map(|(cmd, _)| cmd.as_str());
    // The active panel's selected entry, in full — listing rows truncate
    // long names, so the command bar carries the readable copy.
    let active = if p.active == Side::Left {
        &p.left
    } else {
        &p.right
    };
    let sel_label = bars::selected_label(active);
    command_bar(
        &mut buf,
        split[1],
        &p.active_panel_folder(),
        &p.cmdline,
        ghost.as_deref(),
        ask_hint.as_deref(),
        suggested,
        running,
        sel_label.as_deref(),
    );
    // The make-folder prompt takes over the function-key row while it's open.
    match &p.prompt {
        Some(prompt) => prompt_bar(&mut buf, split[2], prompt),
        None => function_bar(&mut buf, split[2]),
    }
    if let Some(ds) = &p.drive_select {
        drive_select_overlay(&mut buf, area, ds);
    }
    crate::tui::to_cells(&buf)
}

/// The Alt+F1/F2 drive-select overlay: a small centered box listing "Local
/// disk" plus each configured rclone remote, highlighting `sel`. Shows a
/// "listing remotes…" placeholder while `listremotes` is still running
/// (`options` empty).
fn drive_select_overlay(buf: &mut Buffer, area: Rect, ds: &super::remote::DriveSelect) {
    let t = crew_theme::theme();
    let bg = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let ink = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let page_col = bg;
    let rows = ds.options.len().max(1) as u16;
    let h = (rows + 2).min(area.height);
    let w = 32u16.min(area.width);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let box_area = Rect::new(x, y, w, h);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(accent_color()))
        .title(Span::styled(
            "Select drive",
            Style::new().fg(accent_color()),
        ))
        .style(Style::new().bg(bg));
    let inner = block.inner(box_area);
    Widget::render(ratatui::widgets::Clear, box_area, buf);
    block.render(box_area, buf);
    if ds.options.is_empty() {
        Paragraph::new(Line::from(Span::styled(
            "listing remotes\u{2026}",
            Style::new().fg(ink).bg(bg),
        )))
        .style(Style::new().bg(bg))
        .render(inner, buf);
        return;
    }
    let items: Vec<ListItem> = ds
        .options
        .iter()
        .map(|opt| {
            let label = match opt {
                super::remote::DriveOption::Local => "Local disk".to_string(),
                super::remote::DriveOption::Remote(name) => name.clone(),
            };
            ListItem::new(Line::from(Span::styled(label, Style::new().fg(ink).bg(bg))))
        })
        .collect();
    let hl = Style::new().fg(page_col).bg(accent_color());
    let mut state = ListState::default();
    state.select(Some(ds.sel));
    StatefulWidget::render(List::new(items).highlight_style(hl), inner, buf, &mut state);
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
    // The active panel's legend is a FILLED accent tab (the F-key bar's pill
    // language) — the accent border alone was too subtle to tell which side
    // keys act on (user feedback, v0.6.23). Inactive stays plain dim text.
    let legend_style = if active {
        Style::new()
            .fg(page_col)
            .bg(accent_color())
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
    // Only the ACTIVE panel gets a filled cursor bar — with a fill on both
    // sides it was ambiguous which panel keys would act on (the inactive
    // side's bar often sits on `../` and reads as "selected"). The inactive
    // panel remembers its place with a bold row instead of a bar.
    let hl = if active {
        Style::new().fg(page_col).bg(accent_color())
    } else {
        Style::new().add_modifier(Modifier::BOLD)
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

/// `" /path · N · size "` — `N` is the panel's entry count and `size` its
/// total byte size (via `fmt_size`). A directory with zero entries shows
/// `· empty` instead of the (always-zero, redundant) `· 0 · 0 B` — a plain
/// word reads faster than two zeros. The suffix stays intact whenever
/// there's room for it at all; the path truncates from the left (keeping the
/// tail) to fit `width`, same as before the count/size were added.
fn legend(display: &str, count: usize, total: u64, width: u16) -> String {
    let suffix = if count == 0 {
        " \u{00b7} empty ".to_string()
    } else {
        format!(" \u{00b7} {count} \u{00b7} {} ", fmt_size(total))
    };
    let max = (width as usize).saturating_sub(1 + suffix.chars().count());
    if display.chars().count() <= max || max == 0 {
        return format!(" {display}{suffix}");
    }
    let tail: String = display
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
#[path = "bars.rs"]
mod bars;
use bars::{command_bar, function_bar, prompt_bar};

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
