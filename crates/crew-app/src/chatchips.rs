//! The crew pane's boxed agent status cards: one card per agent with an
//! identity line (`▪planner qwen-max`) over a row of rounded, labeled metric
//! boxes (`state`/`tok`/`ctx`/`shr`) — settings-form style. Replaces the old
//! flat text clusters. `layout()` is the single source of truth for how much
//! vertical space the grid needs, shared by `ChatPane::status_rows` (row
//! accounting) and `chatview::cells` (the renderer), so the two can never
//! disagree about the drawn extent (the fix for a confirmed overdraw bug).
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Widget};

use crew_render::CellView;

use crate::chathdr::fmt_tokens;
use crate::chatroster::agent_color;
use crate::chatwidth::str_w;

/// A 2-space gutter between cards on a row.
const GUTTER: usize = 2;
/// Sparsest drop level (marker+name+state box only).
const MAX_LEVEL: u8 = 4;
/// Identity line + a 3-row box (top border, value, bottom border).
pub(crate) const CARD_H: u16 = 4;

/// One agent's snapshot for the grid.
pub(crate) struct AgentView {
    pub name: String,
    pub model: String,
    /// Already-formatted state token (e.g. "⠙3.2s", "·2×", "idle").
    pub state: String,
    pub tok: u64,
    pub ctx_pct: Option<u8>,
    pub share_pct: Option<u8>,
    pub active: bool,
}

/// Computed grid geometry, shared by row accounting and rendering.
pub(crate) struct Layout {
    pub level: u8,
    pub card_w: u16,
    /// Cards per row.
    pub per_row: usize,
    /// Number of card ROWS actually drawn (after capping to `avail_rows`).
    pub card_rows: usize,
    /// `card_rows * CARD_H` — the drawn height of the grid zone.
    pub rows: u16,
}

/// Lay out `views` into `cols` width with at most `avail_rows` rows for the
/// grid zone (the caller has already excluded the session line and
/// waterfall). Caps `card_rows` so `rows <= avail_rows`. `None` when nothing
/// fits (grid hidden, session line only).
pub(crate) fn layout(views: &[AgentView], cols: u16, avail_rows: u16) -> Option<Layout> {
    if views.is_empty() || avail_rows < CARD_H {
        return None;
    }
    let level = choose_level(views, cols)?;
    let card_w = max_card_width(views, level) as u16;
    let per_row = per_row(card_w as usize, cols);
    let need = views.len().div_ceil(per_row);
    let max_card_rows = (avail_rows / CARD_H) as usize;
    let card_rows = need.min(max_card_rows);
    if card_rows == 0 {
        return None;
    }
    Some(Layout {
        level,
        card_w,
        per_row,
        card_rows,
        rows: (card_rows as u16) * CARD_H,
    })
}

/// The metric boxes present at `level`, in display order: label + value.
/// Fields shed from the right as `level` rises: shr, ctx, tok — `state` is
/// always present.
fn metrics_for(v: &AgentView, level: u8) -> Vec<(&'static str, String)> {
    let mut m = vec![("state", v.state.clone())];
    if level < 3 && v.tok > 0 {
        m.push(("tok", fmt_tokens(v.tok)));
    }
    if level < 2 {
        if let Some(p) = v.ctx_pct {
            m.push(("ctx", format!("{p}%")));
        }
    }
    if level < 1 {
        if let Some(p) = v.share_pct {
            m.push(("shr", format!("{p}%")));
        }
    }
    m
}

/// A box's outer width: 1 pad each side of the value + 2 borders, wide enough
/// for the label legend too.
fn box_outer_width(label: &str, value: &str) -> usize {
    str_w(label).max(str_w(value)) + 4
}

/// Sum of the present boxes' outer widths (boxes are adjacent, no gap).
fn box_row_width(v: &AgentView, level: u8) -> usize {
    metrics_for(v, level)
        .iter()
        .map(|(label, value)| box_outer_width(label, value))
        .sum()
}

/// `▪name model` (model dropped at level4).
fn identity_width(v: &AgentView, level: u8) -> usize {
    str_w(&identity_text(v, level))
}

fn identity_text(v: &AgentView, level: u8) -> String {
    let marker = if v.active { '\u{25b8}' } else { '\u{25aa}' };
    let mut s = format!("{marker}{}", v.name);
    if level < MAX_LEVEL && !v.model.is_empty() {
        s.push(' ');
        s.push_str(&v.model);
    }
    s
}

/// A card's width at `level`: the wider of the identity line and the boxes row.
fn card_width(v: &AgentView, level: u8) -> usize {
    identity_width(v, level).max(box_row_width(v, level))
}

/// The widest card across `views` at `level`.
fn max_card_width(views: &[AgentView], level: u8) -> usize {
    views
        .iter()
        .map(|v| card_width(v, level))
        .max()
        .unwrap_or(0)
}

/// The richest level (0 best) whose widest card fits `cols`; `None` if even
/// the sparsest card overflows.
pub(crate) fn choose_level(views: &[AgentView], cols: u16) -> Option<u8> {
    if views.is_empty() {
        return None;
    }
    (0..=MAX_LEVEL).find(|&level| max_card_width(views, level) <= cols as usize)
}

/// Cards that fit on one row of `cols`, given equal `card_w` + gutter.
fn per_row(card_w: usize, cols: u16) -> usize {
    let cols = cols as usize;
    if card_w == 0 || card_w > cols {
        return 1;
    }
    // n cards need n*w + (n-1)*gutter columns.
    ((cols + GUTTER) / (card_w + GUTTER)).max(1)
}

/// Draw the boxed agent cards into a fresh Buffer, convert to cells, offset
/// to `start_row`. Draws exactly `lay.card_rows` rows of cards (already
/// capped by `layout`).
pub(crate) fn grid_cells(
    views: &[AgentView],
    cols: u16,
    start_row: u16,
    lay: &Layout,
) -> Vec<CellView> {
    let area = Rect::new(0, 0, cols, lay.rows);
    let mut buf = Buffer::empty(area);
    let t = crew_theme::theme();
    let dim = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let n = views.len().min(lay.per_row * lay.card_rows);
    for (i, v) in views.iter().take(n).enumerate() {
        let col0 = ((i % lay.per_row) * (lay.card_w as usize + GUTTER)) as u16;
        let row0 = (i / lay.per_row) as u16 * CARD_H;
        draw_identity(&mut buf, v, lay.level, col0, row0, lay.card_w);
        let mut x = col0;
        for (label, value) in metrics_for(v, lay.level) {
            let w = box_outer_width(label, &value) as u16;
            let rect = Rect::new(x, row0 + 1, w, 3);
            draw_box(&mut buf, rect, label, &value, dim);
            x += w;
        }
    }
    let mut cells = crate::tui::to_cells(&buf);
    for c in &mut cells {
        c.row += start_row;
    }
    cells
}

/// Row 0 of a card: marker+name in the agent colour (bold while active),
/// then ` model` in the muted colour (dropped at level4).
fn draw_identity(buf: &mut Buffer, v: &AgentView, level: u8, x: u16, y: u16, max_w: u16) {
    let t = crew_theme::theme();
    let color = agent_color(&v.name);
    let fg = Color::Rgb(color.0, color.1, color.2);
    let muted = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let marker = if v.active { '\u{25b8}' } else { '\u{25aa}' };
    let mut name_style = Style::new().fg(fg);
    if v.active {
        name_style = name_style.add_modifier(Modifier::BOLD);
    }
    let mut spans = vec![Span::styled(format!("{marker}{}", v.name), name_style)];
    if level < MAX_LEVEL && !v.model.is_empty() {
        spans.push(Span::styled(
            format!(" {}", v.model),
            Style::new().fg(muted),
        ));
    }
    buf.set_line(x, y, &Line::from(spans), max_w);
}

/// One rounded metric box: the label as the title legend (settings-form
/// style), the value centered on the middle row.
fn draw_box(buf: &mut Buffer, rect: Rect, label: &str, value: &str, dim: Color) {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(dim))
        .title(Span::styled(format!(" {label} "), Style::new().fg(dim)))
        .render(rect, buf);
    let iw = rect.width.saturating_sub(2) as usize;
    let vw = str_w(value);
    let pad = iw.saturating_sub(vw) / 2;
    let line = Line::styled(value.to_string(), Style::new().fg(dim));
    buf.set_line(rect.x + 1 + pad as u16, rect.y + 1, &line, iw as u16);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(name: &str, active: bool) -> AgentView {
        AgentView {
            name: name.into(),
            model: "qwen-max".into(),
            state: if active {
                "\u{2819}3.2s".into()
            } else {
                "\u{00b7}2\u{00d7}".into()
            },
            tok: 4_100,
            ctx_pct: Some(38),
            share_pct: Some(42),
            active,
        }
    }

    #[test]
    fn choose_level_drops_boxes_as_cols_shrink() {
        let views = vec![v("planner", false)];
        assert_eq!(choose_level(&views, 200), Some(0), "wide → full card");
        // Between the level0 (all 4 boxes) and level1 (drops shr) widths:
        // fits without shr, still has ctx+tok → level >= 1.
        let l = choose_level(&views, 25).expect("still fits a level");
        assert!(l >= 1, "mid width drops the shr box, got {l}");
        // Just wide enough for marker+name + the state box only.
        assert_eq!(
            choose_level(&views, 9),
            Some(4),
            "very narrow-but-fits → sparsest level"
        );
    }

    #[test]
    fn layout_none_when_nothing_fits() {
        assert!(
            layout(&[v("planner", false)], 3, 100).is_none(),
            "too narrow for even the state box"
        );
    }

    #[test]
    fn layout_caps_card_rows_to_available() {
        let views: Vec<AgentView> = (0..6).map(|i| v(&format!("a{i}"), false)).collect();
        // Narrow enough that only one card fits per row.
        let cols = 32;
        assert_eq!(per_row(max_card_width(&views, 0), cols), 1);
        let lay = layout(&views, cols, 8).expect("fits at some level");
        assert_eq!(lay.card_rows, 2, "capped to 8/CARD_H, not the 6 needed");
        assert_eq!(lay.rows, 8);
        assert!(
            layout(&views, cols, 3).is_none(),
            "avail_rows < CARD_H → None"
        );
    }

    #[test]
    fn grid_cells_draws_identity_and_boxed_labels() {
        let views = vec![AgentView {
            name: "planner".into(),
            model: "qwen-max".into(),
            state: "\u{00b7}1\u{00d7}".into(),
            tok: 1_200,
            ctx_pct: Some(3),
            share_pct: Some(100),
            active: false,
        }];
        let cols = 60;
        let lay = layout(&views, cols, 40).expect("fits");
        let start_row = 3;
        let cells = grid_cells(&views, cols, start_row, &lay);
        let text: String = cells.iter().map(|c| c.c).collect();
        assert!(text.contains("planner"), "name: {text}");
        assert!(text.contains("state"), "state label: {text}");
        assert!(text.contains("tok"), "tok label: {text}");
        assert!(text.contains("ctx"), "ctx label: {text}");
        assert!(text.contains("shr"), "shr label: {text}");
        assert!(
            cells.iter().any(|c| c.c == '\u{256d}' || c.c == '\u{2570}'),
            "rounded corner glyph present"
        );
        assert!(text.contains("1.2k"), "tok value: {text}");
        assert!(text.contains("3%"), "ctx value: {text}");
        let max_row = cells.iter().map(|c| c.row).max().unwrap_or(0);
        assert!(
            max_row < start_row + CARD_H,
            "card stays within CARD_H rows: max_row={max_row}"
        );
    }
}
