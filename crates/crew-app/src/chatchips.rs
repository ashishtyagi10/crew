//! The crew pane's agent status rows: one flat, ` │ `-separated line per
//! agent in claude-code's own statusline style — `name │ state │ tok │ <bar>
//! ctx% (ctx) │ <bar> shr% (shr)` — replacing the old boxed metric cards.
//! `layout()` is the single source of truth for how many rows the grid
//! needs, shared by `ChatPane::status_rows` (row accounting) and
//! `chatview::cells` (the renderer), so the two can never disagree about the
//! drawn extent (the fix for a confirmed overdraw bug).
use crew_render::CellView;

use crate::chathdr::fmt_tokens;
use crate::chatroster::agent_color;
use crate::chatwidth::{place_row, str_w};
use crate::gauges::fill_color;

/// Bar width in cells for the ctx/share progress bars.
const BAR_W: usize = 6;
/// Minimum width of the right-aligned `NN%` field (grows past this only for
/// `100%`, which is never truncated).
const PCT_W: usize = 3;
/// The ` │ ` separator between every segment.
const SEP: &str = " \u{2502} ";
const SEP_W: usize = 3;
/// A bar segment's fixed width: `bar + " " + pct(>=PCT_W) + " (" + lbl + ")"`.
const SEG_W: usize = BAR_W + 1 + PCT_W + 1 + 1 + 3 + 1;
/// Sparsest level: name + state only.
const MAX_LEVEL: u8 = 3;

/// One agent's snapshot for the grid.
pub(crate) struct AgentView {
    pub name: String,
    /// Already-formatted state token (e.g. "⠙3.2s", "1×", "idle").
    pub state: String,
    pub tok: u64,
    pub ctx_pct: Option<u8>,
    pub share_pct: Option<u8>,
    pub active: bool,
}

/// Computed row geometry, shared by row accounting and rendering.
pub(crate) struct Layout {
    pub level: u8,
    pub name_w: u16,
    pub state_w: u16,
    pub tok_w: u16,
    /// Agents drawn after the short-pane cap.
    pub shown: usize,
    /// Number of rows actually drawn — one per shown agent.
    pub rows: u16,
}

/// Lay out `views` into `cols` width with at most `avail_rows` rows for the
/// grid zone (the caller has already excluded the session line and
/// waterfall). `None` when nothing fits (grid hidden, session line only).
pub(crate) fn layout(views: &[AgentView], cols: u16, avail_rows: u16) -> Option<Layout> {
    if views.is_empty() || avail_rows == 0 {
        return None;
    }
    let level = choose_level(views, cols)?;
    let shown = views.len().min(avail_rows as usize);
    if shown == 0 {
        return None;
    }
    let subset = &views[..shown];
    Some(Layout {
        level,
        name_w: name_w(subset) as u16,
        state_w: state_w(subset) as u16,
        tok_w: tok_w(subset) as u16,
        shown,
        rows: shown as u16,
    })
}

fn marker(v: &AgentView) -> char {
    if v.active {
        '\u{25b8}'
    } else {
        '\u{25aa}'
    }
}

fn name_text(v: &AgentView) -> String {
    format!("{}{}", marker(v), v.name)
}

/// `tok` formatted, or U+2013 (–) when there is no context to show.
fn tok_text(v: &AgentView) -> String {
    if v.tok == 0 {
        "\u{2013}".into()
    } else {
        fmt_tokens(v.tok)
    }
}

fn name_w(views: &[AgentView]) -> usize {
    views
        .iter()
        .map(|v| str_w(&name_text(v)))
        .max()
        .unwrap_or(0)
}

fn state_w(views: &[AgentView]) -> usize {
    views.iter().map(|v| str_w(&v.state)).max().unwrap_or(0)
}

fn tok_w(views: &[AgentView]) -> usize {
    views.iter().map(|v| str_w(&tok_text(v))).max().unwrap_or(0)
}

/// Total row width at `level`: name+state are always present; tok/ctx/shr
/// shed from the right as `level` rises (shr first, then ctx, then tok).
fn row_width(views: &[AgentView], level: u8) -> usize {
    let mut w = name_w(views) + SEP_W + state_w(views);
    if level <= 2 {
        w += SEP_W + tok_w(views);
    }
    if level <= 1 {
        w += SEP_W + SEG_W;
    }
    if level == 0 {
        w += SEP_W + SEG_W;
    }
    w
}

/// The richest level (0 best) whose row fits `cols`; `None` if even the
/// sparsest row (name+state) overflows.
pub(crate) fn choose_level(views: &[AgentView], cols: u16) -> Option<u8> {
    if views.is_empty() {
        return None;
    }
    (0..=MAX_LEVEL).find(|&level| row_width(views, level) <= cols as usize)
}

/// Left-align `s` to `w` display columns (padding with trailing spaces).
fn pad_left(s: &str, w: usize) -> String {
    let len = str_w(s);
    if len >= w {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(w - len))
    }
}

/// Center `s` within `w` display columns.
fn pad_center(s: &str, w: usize) -> String {
    let len = str_w(s);
    if len >= w {
        return s.to_string();
    }
    let total = w - len;
    let left = total / 2;
    let right = total - left;
    format!("{}{s}{}", " ".repeat(left), " ".repeat(right))
}

/// The two foreground shades a row needs beyond the per-agent colour: `ink`
/// for values, `dim` for separators/labels/track cells. Bundled so the
/// per-segment helpers stay under clippy's argument-count limit.
struct Pal {
    ink: (u8, u8, u8),
    dim: (u8, u8, u8),
}

/// Append `s` at `(row, col..)` in `fg`, stopping before `max_col`. Returns
/// the next free column. Background is always the page background — every
/// cell in this grid sits on the plain page, no card fills.
fn place(
    cells: &mut Vec<CellView>,
    row: u16,
    col: u16,
    max_col: u16,
    s: &str,
    fg: (u8, u8, u8),
    bold: bool,
) -> u16 {
    let bg = crew_theme::theme().page_bg;
    place_row(col, max_col, s.chars().map(|c| (c, fg)), |x, c, fg| {
        cells.push(CellView {
            col: x,
            row,
            c,
            fg,
            bg,
            bold,
            italic: false,
        });
    })
}

/// One `<bar> NN% (label)` segment. `pct = None` draws an all-track bar and
/// ` 0%`.
fn push_segment(
    cells: &mut Vec<CellView>,
    row: u16,
    col: u16,
    max_col: u16,
    pct: Option<u8>,
    label: &str,
    pal: &Pal,
) -> u16 {
    let bg = crew_theme::theme().page_bg;
    let frac = pct.unwrap_or(0) as f32 / 100.0;
    let filled = pct
        .map(|_| (frac * BAR_W as f32).round() as usize)
        .unwrap_or(0);
    let fill = fill_color(frac);
    let mut x = col;
    for i in 0..BAR_W {
        if x >= max_col {
            break;
        }
        let (ch, fg) = if i < filled {
            ('\u{2588}', fill)
        } else {
            ('\u{2591}', pal.dim)
        };
        cells.push(CellView {
            col: x,
            row,
            c: ch,
            fg,
            bg,
            bold: false,
            italic: false,
        });
        x += 1;
    }
    x = place(cells, row, x, max_col, " ", pal.dim, false);
    let pct_str = format!("{:>PCT_W$}", format!("{}%", pct.unwrap_or(0)));
    x = place(cells, row, x, max_col, &pct_str, pal.ink, false);
    place(
        cells,
        row,
        x,
        max_col,
        &format!(" ({label})"),
        pal.dim,
        false,
    )
}

/// Draw the agent status rows, offset to `start_row`. Draws exactly
/// `lay.shown` rows (already capped by `layout`).
pub(crate) fn row_cells(
    views: &[AgentView],
    cols: u16,
    start_row: u16,
    lay: &Layout,
) -> Vec<CellView> {
    let t = crew_theme::theme();
    let pal = Pal {
        ink: t.ink,
        dim: t.text_muted,
    };
    let mut cells = Vec::new();
    for (i, v) in views.iter().take(lay.shown).enumerate() {
        let row = start_row + i as u16;
        let color = agent_color(&v.name);
        let mut col = place(
            &mut cells,
            row,
            0,
            cols,
            &pad_left(&name_text(v), lay.name_w as usize),
            color,
            v.active,
        );
        col = place(&mut cells, row, col, cols, SEP, pal.dim, false);
        col = place(
            &mut cells,
            row,
            col,
            cols,
            &pad_center(&v.state, lay.state_w as usize),
            pal.dim,
            false,
        );
        if lay.level <= 2 {
            col = place(&mut cells, row, col, cols, SEP, pal.dim, false);
            col = place(
                &mut cells,
                row,
                col,
                cols,
                &pad_center(&tok_text(v), lay.tok_w as usize),
                pal.dim,
                false,
            );
        }
        if lay.level <= 1 {
            col = place(&mut cells, row, col, cols, SEP, pal.dim, false);
            col = push_segment(&mut cells, row, col, cols, v.ctx_pct, "ctx", &pal);
        }
        if lay.level == 0 {
            col = place(&mut cells, row, col, cols, SEP, pal.dim, false);
            col = push_segment(&mut cells, row, col, cols, v.share_pct, "shr", &pal);
        }
        let _ = col;
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(
        name: &str,
        state: &str,
        tok: u64,
        ctx: Option<u8>,
        share: Option<u8>,
        active: bool,
    ) -> AgentView {
        AgentView {
            name: name.into(),
            state: state.into(),
            tok,
            ctx_pct: ctx,
            share_pct: share,
            active,
        }
    }

    fn trio() -> Vec<AgentView> {
        vec![
            v("planner", "1\u{d7}", 5_800, Some(17), Some(100), true),
            v("coder", "idle", 0, None, None, false),
            v("reviewer", "idle", 0, None, None, false),
        ]
    }

    #[test]
    fn layout_one_row_per_agent_capped() {
        let views = trio();
        let lay = layout(&views, 200, 100).expect("wide/tall pane fits everything");
        assert_eq!(lay.rows, 3, "one row per agent");
        assert_eq!(lay.shown, 3);

        let lay2 = layout(&views, 200, 2).expect("capped pane still fits some rows");
        assert_eq!(lay2.shown, 2);
        assert_eq!(lay2.rows, 2);

        assert!(layout(&views, 200, 0).is_none(), "no available rows → None");
    }

    #[test]
    fn choose_level_drops_segments_right_to_left() {
        let views = trio();
        assert_eq!(choose_level(&views, 200), Some(0), "wide → every segment");

        // Wide enough to lose only the shr segment (level 1: name/state/tok/ctx).
        let level1_w = row_width(&views, 1);
        let level0_w = row_width(&views, 0);
        assert!(level1_w < level0_w, "dropping shr must shrink the row");
        let l = choose_level(&views, level1_w as u16).expect("fits at level 1");
        assert!(l >= 1, "mid width drops the shr segment, got {l}");

        // Just wide enough for name+state only (the sparsest level that fits).
        let level3_w = row_width(&views, MAX_LEVEL);
        assert_eq!(
            choose_level(&views, level3_w as u16),
            Some(MAX_LEVEL),
            "narrow-but-fits → sparsest level"
        );

        assert_eq!(choose_level(&views, 3), None, "too narrow for anything");
    }

    #[test]
    fn row_cells_have_name_state_pipes_and_ctx_shr_bars() {
        let views = trio();
        let cols = 200;
        let lay = layout(&views, cols, 100).expect("fits");
        let start_row = 1;
        let cells = row_cells(&views, cols, start_row, &lay);

        let text_at = |row: u16| -> String {
            let mut row_cells: Vec<(u16, char)> = cells
                .iter()
                .filter(|c| c.row == row)
                .map(|c| (c.col, c.c))
                .collect();
            row_cells.sort();
            row_cells.into_iter().map(|(_, c)| c).collect()
        };
        let (planner, coder, reviewer) = (
            text_at(start_row),
            text_at(start_row + 1),
            text_at(start_row + 2),
        );
        assert!(planner.contains("planner"), "planner name: {planner}");
        assert!(coder.contains("coder"), "coder name: {coder}");
        assert!(reviewer.contains("reviewer"), "reviewer name: {reviewer}");

        assert!(planner.contains("(ctx)"), "ctx label: {planner}");
        assert!(planner.contains("(shr)"), "shr label: {planner}");

        // Separators land at the same columns across every row (leading-column
        // alignment).
        let pipe_cols = |row: u16| -> Vec<u16> {
            let mut cols: Vec<u16> = cells
                .iter()
                .filter(|c| c.row == row && c.c == '\u{2502}')
                .map(|c| c.col)
                .collect();
            cols.sort();
            cols
        };
        let p0 = pipe_cols(start_row);
        assert!(!p0.is_empty(), "planner row has separators");
        assert_eq!(
            p0,
            pipe_cols(start_row + 1),
            "coder pipes align with planner's"
        );
        assert_eq!(
            p0,
            pipe_cols(start_row + 2),
            "reviewer pipes align with planner's"
        );

        // Active agent (ctx=17%) has both filled and track cells in its bars.
        assert!(cells
            .iter()
            .any(|c| c.row == start_row && c.c == '\u{2588}'));
        assert!(cells
            .iter()
            .any(|c| c.row == start_row && c.c == '\u{2591}'));

        // Idle agent's ctx bar is all-track (no fill) since ctx_pct is None.
        let coder_row = start_row + 1;
        assert!(
            !cells
                .iter()
                .any(|c| c.row == coder_row && c.c == '\u{2588}'),
            "idle agent has no filled bar cells: {coder}"
        );
        assert!(cells
            .iter()
            .any(|c| c.row == coder_row && c.c == '\u{2591}'));

        // Idle agent's tok column renders the dash placeholder.
        assert!(coder.contains('\u{2013}'), "idle tok is a dash: {coder}");
    }
}
