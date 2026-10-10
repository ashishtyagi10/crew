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
    let dim_col = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let text_col = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let edge = if active && focused {
        accent_color()
    } else {
        dim_col
    };
    // The active panel's legend is bold words — accent while the keys are
    // here, ink while they are elsewhere — and its frame is the accent. It
    // was a FILLED accent tab (v0.6.23: the accent border alone was too
    // subtle); on the glass the tab and the cursor bar under it stood as one
    // stepped pink slab in dark ink, the loudest thing on the screen (glass
    // survey C#2). The bar alone carries the fill now. Inactive: dim text.
    let legend_style = match (active, focused) {
        (true, true) => Style::new().fg(accent_color()),
        (true, false) => Style::new().fg(text_col),
        (false, _) => Style::new().fg(dim_col),
    };
    let legend_style = match active {
        true => legend_style.add_modifier(Modifier::BOLD),
        false => legend_style,
    };
    // One cell of rule before the tab, as every card's `╭─ legend` keeps: the
    // corner rounds through it (`crew_render`'s card-scale corners need a
    // plain `─` to bend into, and a tab against the `╭` left it square).
    let tab = legend(
        &panel.loc.shown(),
        panel.entries.len(),
        panel.entries.iter().map(|e| e.size).sum::<u64>(),
        area.width.saturating_sub(1),
    );
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(edge))
        .title(Line::from(vec![
            Span::styled("\u{2500}", Style::new().fg(edge)),
            Span::styled(tab, legend_style),
        ]));
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
            // A column short of the border: a size flush against the divider
            // or the frame (`3.7K│`) read as part of the line.
            let width = inner.width.saturating_sub(1) as usize;
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
    // panel remembers its place with a bold row instead of a bar; the bar is
    // bold too (see `on_accent`).
    let hl = match (active, focused) {
        (true, true) => on_accent(),
        (true, false) => blurred_bar(),
        (false, _) => Style::new().add_modifier(Modifier::BOLD),
    };
    let mut state = ListState::default();
    state.select(Some(panel.sel - start));
    StatefulWidget::render(List::new(items).highlight_style(hl), inner, buf, &mut state);
}

/// The cursor bar, a landed suggestion and the drive list's choice: bold
/// white on the accent deepened until white reads on it
/// ([`crate::segment::inked`]) — the page's dark smoke on the bright accent
/// was the only dark lettering left on the glass (survey C#2). Bold, like
/// the path tab: on a phosphor tube the glow round a bright fill swallowed a
/// regular-weight `../` whole.
pub(super) fn on_accent() -> Style {
    let (ink, fill) = crate::segment::inked(crate::palette::accent());
    Style::new()
        .fg(rgb(ink))
        .bg(rgb(fill))
        .add_modifier(Modifier::BOLD)
}

/// The cursor bar of a pane the keys are not in: the place kept in a quiet
/// step of the smoke toward the ink, which the glass draws as a lighter
/// frost of itself — the selection's navy there was a solid slab on any
/// desktop, off every palette but one (survey C#2).
pub(super) fn blurred_bar() -> Style {
    let t = crew_theme::theme();
    let fill = crate::anim::lerp_rgb(t.page_bg, t.ink, BLURRED_BAR_SHADE);
    let ink = crew_theme::readable::enforced(t.ink, fill, crew_theme::contrast::text_floor());
    Style::new()
        .fg(rgb(ink))
        .bg(rgb(fill))
        .add_modifier(Modifier::BOLD)
}

/// How far the blurred bar sits off the page toward the ink: the settings
/// form's quiet Cancel capsule.
pub(super) const BLURRED_BAR_SHADE: f32 = 0.14;

fn rgb((r, g, b): (u8, u8, u8)) -> Color {
    Color::Rgb(r, g, b)
}

#[cfg(test)]
#[path = "rowair_tests.rs"]
mod tests;
