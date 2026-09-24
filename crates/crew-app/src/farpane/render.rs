//! Dual-pane file-manager rendering: two bordered directory panels side by side
//! (the active one accent-bordered, its cursor highlighted) over a Far-style
//! function-key bar. Built with ratatui and handed to the GPU as cells.
pub(crate) use super::panelchrome::*;
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

#[cfg(test)]
pub(crate) fn render(p: &FarPane, cols: u16, rows: u16) -> Vec<CellView> {
    render_in(p, cols, rows, true)
}

/// A pane that is not focused shows where it was, not that it is listening:
/// no caret on its command line, and its cursor bar and path tab in the
/// selection wash instead of the accent (the way a Mac's inactive window
/// greys its selection) — with two `/far` panes open both used to look live.
pub(crate) fn render_in(p: &FarPane, cols: u16, rows: u16, focused: bool) -> Vec<CellView> {
    if cols < 16 || rows < 6 {
        return Vec::new();
    }
    let area = Rect::new(0, 0, cols, rows);
    let mut buf = Buffer::empty(area);
    // Panels, then the status line (selected entry in full), then the
    // command line, then the function-key bar.
    let split = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area);
    let (larea, rarea) = split_panels(split[0]);
    panel(&mut buf, larea, &p.left, p.active == Side::Left, focused);
    panel(&mut buf, rarea, &p.right, p.active == Side::Right, focused);
    merge_divider(&mut buf, split[0], rarea.x, focused);
    // Scroll thumbs paint last: the left panel's border is the shared middle
    // column, which the right panel's block render and merge_divider both
    // overwrite — so a thumb drawn inside panel() would be lost.
    scroll_thumb(&mut buf, larea, &p.left, focused && p.active == Side::Left);
    scroll_thumb(
        &mut buf,
        rarea,
        &p.right,
        focused && p.active == Side::Right,
    );
    // A Tab-cycle already shows its candidate in `cmdline` directly; the
    // ghost slot carries the candidate strip instead (`CycleState::hint`),
    // so the next Tab is a choice, not a guess.
    let ghost = match &p.complete {
        None => p
            .history
            .ghost(&p.cmdline)
            .map(|full| full[p.cmdline.len()..].to_string()),
        Some(cycle) => Some(cycle.hint()),
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
    // long names, so the status row carries the readable copy.
    let active = if p.active == Side::Left {
        &p.left
    } else {
        &p.right
    };
    let sel_label = bars::selected_label(active);
    status_bar(&mut buf, split[1], sel_label.as_deref());
    command_bar(
        &mut buf,
        split[2],
        &p.active_panel_folder(),
        &format!("{}{}", p.cmdline, if focused { "\u{258f}" } else { "" }),
        ghost.as_deref(),
        ask_hint.as_deref(),
        suggested,
        running,
    );
    // The make-folder prompt takes over the function-key row while it's open.
    match &p.prompt {
        Some(prompt) => prompt_bar(&mut buf, split[3], prompt),
        None => function_bar(&mut buf, split[3]),
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

#[path = "panellist.rs"]
mod panellist;
use panellist::panel;

#[path = "bars.rs"]
mod bars;
#[path = "rowfit.rs"]
mod rowfit;
use bars::{command_bar, function_bar, prompt_bar, status_bar};

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "blur_tests.rs"]
mod blur_tests;
