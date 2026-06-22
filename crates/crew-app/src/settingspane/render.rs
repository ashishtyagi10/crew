//! Settings form rendering, built on ratatui's layout engine + widgets and
//! converted to `CellView`s. Crew still draws the GPU pane border around this;
//! ratatui owns the in-pane structure (boxes, list, buttons).
use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Clear, List, ListItem, ListState, Paragraph, StatefulWidget, Widget,
};

use super::{Field, SettingsPane};

const ACCENT: Color = Color::Rgb(0, 255, 160);
const TEXT: Color = Color::Rgb(200, 200, 200);
const DIM: Color = Color::Rgb(120, 130, 140);
const BG: Color = Color::Rgb(0, 0, 0);

/// Render the form into a ratatui buffer, then hand the cells to the GPU.
pub(crate) fn render(p: &SettingsPane, cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 24 || rows < 6 {
        return Vec::new();
    }
    let area = Rect::new(0, 0, cols, rows);
    let mut buf = Buffer::empty(area);

    let (fr, btn) = field_layout(area);
    let f = p.focused_field();
    let nav = if p.draft.show_nav { "on" } else { "off" };
    input_box(
        &mut buf,
        fr[0],
        "Font family",
        &p.family_query,
        f == Field::FontFamily,
        true,
    );
    input_box(
        &mut buf,
        fr[1],
        "Font size",
        &p.size_buf,
        f == Field::FontSize,
        true,
    );
    input_box(
        &mut buf,
        fr[2],
        "Nav width",
        &p.nav_buf,
        f == Field::NavWidth,
        true,
    );
    input_box(&mut buf, fr[3], "Show nav", nav, f == Field::ShowNav, false);
    buttons(&mut buf, btn, f);

    if p.family_open {
        dropdown(&mut buf, p, fr[0]);
    }
    crate::tui::to_cells(&buf)
}

/// Width at/above which the form uses two columns.
const WIDE: u16 = 76;

/// Lay out the four field boxes and the buttons row, responsively: two columns
/// on a wide pane (family/size left, nav/show-nav right), one column otherwise.
/// Returns the field rects in `Field` order plus the buttons rect.
fn field_layout(area: Rect) -> ([Rect; 4], Rect) {
    let main = Layout::vertical([
        Constraint::Length(1), // top padding
        Constraint::Min(0),    // fields
        Constraint::Length(1), // gap
        Constraint::Length(1), // buttons
    ])
    .horizontal_margin(2)
    .split(area);
    let (body, btn) = (main[1], main[3]);

    if area.width >= WIDE {
        let halves = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Length(2),
            Constraint::Percentage(50),
        ])
        .split(body);
        let l = two_boxes(halves[0]);
        let r = two_boxes(halves[2]);
        ([l[0], l[1], r[0], r[1]], btn)
    } else {
        let v = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(body);
        ([v[0], v[2], v[4], v[6]], btn)
    }
}

/// Split a column into two stacked 3-row box rects (with a 1-row gap).
fn two_boxes(col: Rect) -> [Rect; 2] {
    let v = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .split(col);
    [v[0], v[2]]
}

/// A rounded input box (field name as the legend) with the value inside.
fn input_box(buf: &mut Buffer, area: Rect, legend: &str, value: &str, focused: bool, cursor: bool) {
    let edge = if focused { ACCENT } else { DIM };
    let fg = if focused { ACCENT } else { TEXT };
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(edge))
        .title(Span::styled(format!(" {legend} "), Style::new().fg(fg)));
    let inner = block.inner(area);
    block.render(area, buf);
    let mut val = value.to_string();
    if focused && cursor {
        val.push('█');
    }
    Paragraph::new(Line::styled(val, Style::new().fg(fg))).render(inner, buf);
}

/// `[ Save ]   [ Cancel ]`, the focused one accent + bold.
fn buttons(buf: &mut Buffer, area: Rect, f: Field) {
    let line = Line::from(vec![
        button_span("[ Save ]", f == Field::Save),
        Span::raw("   "),
        button_span("[ Cancel ]", f == Field::Cancel),
    ]);
    Paragraph::new(line).render(area, buf);
}

fn button_span(text: &str, focused: bool) -> Span<'static> {
    let mut style = Style::new().fg(if focused { ACCENT } else { DIM });
    if focused {
        style = style.add_modifier(Modifier::BOLD);
    }
    Span::styled(text.to_string(), style)
}

/// The type-to-search font list, drawn as a rounded popup anchored below the
/// family box (over the rows beneath it).
fn dropdown(buf: &mut Buffer, p: &SettingsPane, anchor: Rect) {
    let names = p.filtered();
    let want = names.len() as u16 + 2;
    let y0 = anchor.y + anchor.height; // just below the family box
    let max = buf.area.height.saturating_sub(y0);
    let height = want.clamp(3, max.max(3));
    let area = Rect::new(anchor.x, y0, anchor.width, height.min(max));
    Clear.render(area, buf);
    let items: Vec<ListItem> = names.into_iter().map(ListItem::new).collect();
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT))
        .title(Span::styled(" fonts ", Style::new().fg(ACCENT)));
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::new().fg(BG).bg(ACCENT))
        .highlight_symbol("› ");
    let mut state = ListState::default();
    state.select(Some(p.family_sel));
    StatefulWidget::render(list, area, buf, &mut state);
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
