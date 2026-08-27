//! Keyboard-shortcuts overlay (`/keys`): a centered rounded popup listing the
//! bindings, rendered with ratatui and dismissed by any key press.
use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, Widget};

use crate::palette::accent_color;

use crate::helptable::{
    BINDINGS, CHAT_BINDINGS, FAR_BINDINGS, SETTINGS_BINDINGS, TODO_BINDINGS, VIEW_BINDINGS,
};

/// Width of the key column. Every description starts here.
const KEY_COL: usize = 26;

/// Preferred overlay size in cells: both binding tables, a spacer and a
/// heading between them, plus borders/title/hint.
///
/// It used to include every slash command as well, which made 58 rows —
/// taller than a default window, so the commands were silently cut off at
/// the bottom. They are not here any more: the composer's palette has listed
/// them since v0.6.52, grouped and filterable, which is strictly better than
/// an unscrollable overflowing column. `/keys` is for keys.
///
/// The WIDTH is measured the same way, and was not: it was the constant 58,
/// which left 30 columns for a description, and eight rows had outgrown that.
/// ratatui clips a `Line` without a word of complaint, so `Esc` read "Discard
/// a pending plan · inter" — losing both the interrupt and the close — and
/// `@a+b` lost "in parallel", which is the entire point of the binding. The
/// same lesson as the footer in v0.6.57: a truncated instruction teaches the
/// half that fits, and nobody can see that the rest existed.
pub fn size() -> (u16, u16) {
    let rows = sections()
        .iter()
        .map(|(_, table)| table.len() + 2)
        .sum::<usize>()
        + BINDINGS.len()
        + 4;
    let widest = BINDINGS
        .iter()
        .chain(CHAT_BINDINGS)
        .chain(VIEW_BINDINGS)
        .chain(FAR_BINDINGS)
        .chain(TODO_BINDINGS)
        .chain(SETTINGS_BINDINGS)
        .map(|(k, d)| KEY_COL.max(k.chars().count() + 1) + d.chars().count())
        .max()
        .unwrap_or(KEY_COL);
    ((widest + 2) as u16, rows as u16)
}

/// Every row of the overlay, in order, as `(keys, description)` — with the
/// spacer and the "in an agent pane" heading in their places. One list, so
/// scrolling has something to index and the tests have something to count.
/// The per-pane sections, in the order they are listed. One place, so
/// adding a pane kind is one row here and nothing else — the height, the
/// width, the scrolling and the filter all read this.
pub(crate) fn sections() -> [(&'static str, &'static [(&'static str, &'static str)]); 5] {
    [
        ("in an agent pane", CHAT_BINDINGS),
        ("in the file viewer", VIEW_BINDINGS),
        ("in a /far file panel", FAR_BINDINGS),
        ("in the /todo pane", TODO_BINDINGS),
        ("in /settings", SETTINGS_BINDINGS),
    ]
}

fn lines() -> Vec<(&'static str, &'static str)> {
    let mut v: Vec<(&str, &str)> = BINDINGS.to_vec();
    for (title, table) in sections() {
        v.push(("", ""));
        v.push(("", title));
        v.extend_from_slice(table);
    }
    v
}

/// The rows a search shows: every binding whose keys or description contain
/// `needle`, case-insensitively.
///
/// A heading survives only when something under it did — a section title over
/// no rows is a lie about where you are in the list — and the blank spacers go
/// with them, since a filtered list has nothing to separate.
fn filtered(needle: &str) -> Vec<(&'static str, &'static str)> {
    let all = lines();
    if needle.is_empty() {
        return all;
    }
    let n = needle.to_lowercase();
    let hit = |(k, d): &(&str, &str)| {
        !k.is_empty() && (k.to_lowercase().contains(&n) || d.to_lowercase().contains(&n))
    };
    let mut out: Vec<(&'static str, &'static str)> = Vec::new();
    for (i, row) in all.iter().enumerate() {
        if hit(row) {
            // Carry the heading this row sits under, once.
            if let Some(head) = all[..i]
                .iter()
                .rev()
                .find(|(k, d)| k.is_empty() && !d.is_empty())
            {
                if !out.contains(head) {
                    out.push(*head);
                }
            }
            out.push(*row);
        }
    }
    out
}

/// Rows of the list the overlay can show at `rows` cells tall (borders take
/// two). Zero when there is no room for a single row.
fn visible_rows(rows: u16) -> usize {
    usize::from(rows.saturating_sub(2))
}

/// The furthest the overlay can be scrolled: enough to bring the last row
/// into view, and not one row more — scrolling into blank space below a list
/// is the thing that makes an unfamiliar scroll feel broken.
pub fn max_scroll(rows: u16, needle: &str) -> usize {
    filtered(needle).len().saturating_sub(visible_rows(rows))
}

/// Render the help overlay into a `cols × rows` grid, starting `scroll` rows
/// down the list.
///
/// It did not used to scroll, which made its height a hard budget: the list
/// had to fit a default window or ratatui cut the bottom off in silence, and
/// `the_overlay_fits_a_default_window` enforced that by failing whenever a
/// binding was added. Three times in one release the fix was to *merge two
/// rows* to make room — losing detail from bindings that had nothing to do
/// with the new one. A list that scrolls has no budget to spend.
pub fn help_cells(cols: u16, rows: u16, scroll: usize, needle: &str) -> Vec<CellView> {
    if cols < 12 || rows < 4 {
        return Vec::new();
    }
    let scroll = scroll.min(max_scroll(rows, needle));
    let t = crew_theme::theme();
    let text_col = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let dim_col = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let panel_col = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let mut buf = Buffer::empty(Rect::new(0, 0, cols, rows));
    let item = |left: &str, right: &str| {
        ListItem::new(Line::from(vec![
            Span::styled(
                format!("{left:<width$}", width = KEY_COL),
                Style::new().fg(accent_color()),
            ),
            Span::styled(right.to_string(), Style::new().fg(text_col)),
        ]))
    };
    let all = filtered(needle);
    let mut items: Vec<ListItem> = all[scroll.min(all.len())..]
        .iter()
        .map(|&(k, d)| match (k, d) {
            ("", "") => ListItem::new(Line::from("")), // spacer
            ("", head) => ListItem::new(Line::from(Span::styled(
                head.to_string(),
                Style::new().fg(dim_col),
            ))),
            (k, d) => item(k, d),
        })
        .collect();
    // A search that matches nothing must say so; an empty panel reads as a
    // rendering fault.
    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("no binding matches \"{needle}\""),
            Style::new().fg(dim_col),
        ))));
    }
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(accent_color()))
        .style(Style::new().bg(panel_col))
        .title(Span::styled(
            match needle.is_empty() {
                true => format!(" keys \u{b7} crew v{} ", env!("CARGO_PKG_VERSION")),
                // What you typed, shown where the version was: a filter you
                // cannot see is a list that looks broken.
                false => format!(" keys \u{b7} {needle}\u{2588} "),
            },
            Style::new().fg(accent_color()),
        ));
    let inner = block.inner(buf.area);
    block.render(buf.area, &mut buf);
    List::new(items).render(inner, &mut buf);
    // Dismissal hint on the bottom border — and, while there is more list
    // than window, how to reach the rest of it. A scrollable thing that never
    // says so is one nobody scrolls.
    let more = scroll < max_scroll(rows, needle);
    let hint = match (scroll > 0, more) {
        (_, true) => " \u{2191}\u{2193} for more \u{b7} type to filter \u{b7} esc ",
        (true, false) => " \u{2191} for the rest \u{b7} type to filter \u{b7} esc ",
        _ => " type to filter \u{b7} esc to close ",
    };
    let hint_col = cols.saturating_sub(hint.chars().count() as u16 + 2);
    for (i, ch) in hint.chars().enumerate() {
        let col = hint_col + i as u16;
        if let Some(cell) = buf.cell_mut((col, rows - 1)) {
            cell.set_char(ch).set_fg(dim_col);
        }
    }
    crate::tui::to_cells_opaque(&buf)
}

/// How many rows a page key moves, so a long list is a few presses rather
/// than thirty.
const PAGE: i32 = 8;

impl crate::app::CrewApp {
    /// The rows a key moves the open help by, or `None` when the key means
    /// "close" — which is every key that is not a way of moving through a
    /// list, so the overlay keeps its press-anything-to-dismiss habit.
    pub(crate) fn help_scroll_step(&self, key: &winit::keyboard::Key) -> Option<i32> {
        use winit::keyboard::{Key, NamedKey};
        match key {
            Key::Named(NamedKey::ArrowDown) => Some(1),
            Key::Named(NamedKey::ArrowUp) => Some(-1),
            Key::Named(NamedKey::PageDown) => Some(PAGE),
            Key::Named(NamedKey::PageUp) => Some(-PAGE),
            Key::Named(NamedKey::End) => Some(i32::MAX),
            Key::Named(NamedKey::Home) => Some(i32::MIN),
            _ => None,
        }
    }

    /// One key press while the overlay is open.
    ///
    /// Typing filters rather than dismissing: with forty-odd bindings the list
    /// is a document, and the fastest way through a document is to say what
    /// you are looking for. That leaves Esc (and Enter) to close it, which is
    /// what a person who typed something expects anyway; every other key still
    /// closes, so nothing traps you in here.
    pub(crate) fn help_key(&mut self, key: &winit::keyboard::Key) {
        use winit::keyboard::{Key, NamedKey};
        if let Some(step) = self.help_scroll_step(key) {
            self.scroll_help(step);
            return;
        }
        match key {
            Key::Character(c) if !c.is_empty() && !c.chars().any(char::is_control) => {
                self.help_filter.push_str(c);
                // A narrower list read from wherever the old one was scrolled
                // to would open on nothing.
                self.help_scroll = 0;
                return;
            }
            Key::Named(NamedKey::Space) => {
                self.help_filter.push(' ');
                self.help_scroll = 0;
                return;
            }
            Key::Named(NamedKey::Backspace) => {
                self.help_filter.pop();
                self.help_scroll = 0;
                return;
            }
            _ => {}
        }
        self.help_open = false;
        self.help_scroll = 0;
        self.help_filter.clear();
    }

    /// Move the open help by `step` rows, clamped to its list.
    pub(crate) fn scroll_help(&mut self, step: i32) {
        let rows = self
            .frame_geometry()
            .map_or(size().1, |(_, ch, _, sh, _)| (sh / ch) as u16)
            .min(size().1);
        let max = max_scroll(rows, &self.help_filter) as i64;
        let want = self.help_scroll as i64 + i64::from(step);
        self.help_scroll = want.clamp(0, max) as usize;
    }
}

#[cfg(test)]
#[path = "help_tests.rs"]
mod tests;
