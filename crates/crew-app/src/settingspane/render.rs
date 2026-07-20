//! Settings form rendering: a two-column bento of fieldset cards
//! (Appearance / Window / Notifications) with boxed inputs, checkboxes and a
//! notify-patterns text area, plus the font dropdown popup and a pinned
//! Save/Cancel row. Built on ratatui and converted to `CellView`s; Crew
//! draws the GPU pane border around it.
use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::{form, Field, SettingsPane};
use crate::palette::accent_color;

pub(crate) fn label_of(f: Field) -> &'static str {
    match f {
        Field::FontFamily => "Font family",
        Field::FontSize => "Font size",
        Field::NavWidth => "Nav width",
        Field::ShowNav => "Show nav",
        Field::Theme => "Theme",
        Field::Accent => "Accent (#hex)",
        Field::PaperTexture => "Paper texture",
        Field::PaperGrain => "Grain (0-2)",
        Field::Maximized => "Launch maximized",
        Field::Notify => "Notifications",
        Field::NotifyAgentDone => "Notify: cmd done",
        Field::NotifyBell => "Notify: bell",
        Field::NotifyExit => "Notify: pane exit",
        Field::NotifyMinSecs => "Min secs",
        Field::NotifyPatterns => "Patterns (one per line)",
        Field::Save | Field::Cancel => "",
    }
}

/// The value text shown for a field, and whether it takes a typing cursor.
pub(crate) fn value_of(p: &SettingsPane, f: Field) -> (String, bool) {
    let onoff = |b: bool| (if b { "on" } else { "off" }).to_string();
    match f {
        Field::FontFamily => (p.family_query.clone(), true),
        Field::FontSize => (p.size_buf.clone(), true),
        Field::NavWidth => (p.nav_buf.clone(), true),
        Field::ShowNav => (onoff(p.draft.show_nav), false),
        Field::Theme => (
            format!("\u{2039} {} \u{203a}", p.draft.theme_label()),
            false,
        ),
        Field::Accent => (p.accent_buf.clone(), true),
        Field::PaperTexture => (onoff(p.draft.paper_texture), false),
        Field::PaperGrain => (p.grain_buf.clone(), true),
        Field::Maximized => (onoff(p.draft.maximized), false),
        Field::Notify => (onoff(p.draft.notify), false),
        Field::NotifyAgentDone => (onoff(p.draft.notify_agent_done), false),
        Field::NotifyBell => (onoff(p.draft.notify_bell), false),
        Field::NotifyExit => (onoff(p.draft.notify_exit), false),
        Field::NotifyMinSecs => (p.minsecs_buf.clone(), true),
        Field::NotifyPatterns => (p.patterns_buf.clone(), true),
        Field::Save | Field::Cancel => (String::new(), false),
    }
}

/// Render the form into a ratatui buffer, then hand the cells to the GPU.
pub(crate) fn render(p: &SettingsPane, cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 24 || rows < 6 {
        return Vec::new();
    }
    let lay = form::layout(cols);
    let mut virt = Buffer::empty(Rect::new(0, 0, cols, lay.height.max(1)));
    let focused = p.focused_field();
    let frect = lay.rect_of(focused);
    for c in &lay.cards {
        let active = frect.is_some_and(|r| c.rect.intersects(r));
        form::card(&mut virt, c, active);
    }
    for &(f, r) in &lay.rects {
        control(&mut virt, p, f, r, f == focused);
    }

    let viewport = rows.saturating_sub(2); // gap + button row
                                           // Save/Cancel live on the pinned button row: keep the form tail visible.
    let tail = Rect::new(0, lay.height.saturating_sub(1), 1, 1);
    let off = form::scroll_for(frect.unwrap_or(tail), lay.height, viewport);

    let mut out = Buffer::empty(Rect::new(0, 0, cols, rows));
    blit(&mut out, &virt, off, viewport);
    hints(&mut out, cols, viewport, off, lay.height);
    buttons(&mut out, cols, rows, focused);
    if p.family_open {
        if let Some(r) = lay.rect_of(Field::FontFamily) {
            if r.y >= off {
                super::dropdown::dropdown(
                    &mut out,
                    p,
                    Rect::new(r.x, r.y - off, r.width, r.height),
                );
            }
        }
    }
    crate::tui::to_cells(&out)
}

/// Draw one field's control into the virtual buffer.
fn control(buf: &mut Buffer, p: &SettingsPane, f: Field, r: Rect, focused: bool) {
    let d = &p.draft;
    let check = |buf: &mut Buffer, on| form::checkbox(buf, r, label_of(f), on, focused);
    match f {
        Field::ShowNav => check(buf, d.show_nav),
        Field::PaperTexture => check(buf, d.paper_texture),
        Field::Maximized => check(buf, d.maximized),
        Field::Notify => check(buf, d.notify),
        Field::NotifyAgentDone => check(buf, d.notify_agent_done),
        Field::NotifyBell => check(buf, d.notify_bell),
        Field::NotifyExit => check(buf, d.notify_exit),
        Field::NotifyPatterns => form::text_area(buf, r, label_of(f), &p.patterns_buf, focused),
        Field::Save | Field::Cancel => {}
        _ => {
            let (value, cursor) = value_of(p, f);
            form::input_box(buf, r, label_of(f), &value, focused, cursor);
        }
    }
}

/// Copy `viewport` rows of `src` starting at virtual row `off` into `dst`.
fn blit(dst: &mut Buffer, src: &Buffer, off: u16, viewport: u16) {
    let cols = dst.area.width;
    let rows = viewport.min(src.area.height.saturating_sub(off));
    for y in 0..rows {
        for x in 0..cols {
            if let (Some(s), Some(d)) = (src.cell((x, y + off)), dst.cell_mut((x, y))) {
                *d = s.clone();
            }
        }
    }
}

/// `↑` / `↓` markers at the right edge when the form overflows the viewport.
fn hints(buf: &mut Buffer, cols: u16, viewport: u16, off: u16, total: u16) {
    let style = Style::new().fg(form::dim());
    if off > 0 {
        buf.set_line(cols - 2, 0, &Line::styled("\u{2191}", style), 1);
    }
    if off + viewport < total {
        let y = viewport.saturating_sub(1);
        buf.set_line(cols - 2, y, &Line::styled("\u{2193}", style), 1);
    }
}

/// `[ Save ⌘S ]   [ Cancel esc ]`, pinned bottom-right, focused accent+bold.
fn buttons(buf: &mut Buffer, cols: u16, rows: u16, f: Field) {
    let (save, cancel) = ("[ Save \u{2318}S ]", "[ Cancel esc ]");
    let w = (save.chars().count() + 3 + cancel.chars().count()) as u16;
    let line = Line::from(vec![
        button_span(save, f == Field::Save),
        Span::raw("   "),
        button_span(cancel, f == Field::Cancel),
    ]);
    buf.set_line(cols.saturating_sub(w + 2), rows - 1, &line, w);
}

fn button_span(text: &str, focused: bool) -> Span<'static> {
    let t = crew_theme::theme();
    let dim_col = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let mut style = Style::new().fg(if focused { accent_color() } else { dim_col });
    if focused {
        style = style.add_modifier(Modifier::BOLD);
    }
    Span::styled(text.to_string(), style)
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
