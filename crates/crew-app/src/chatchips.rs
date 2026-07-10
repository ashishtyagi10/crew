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
/// Width of the right-aligned `NNN%` field — wide enough for `100%` (4
/// chars) so the trailing `)` of the `(ctx)`/`(shr)` label never clips.
const PCT_W: usize = 4;
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
    /// Eased ctx fill fraction (0.0..1.0) — drives the bar; `ctx_pct` still
    /// drives the target percentage text.
    pub ctx_frac: f32,
    /// Eased share fill fraction (0.0..1.0) — drives the bar; `share_pct`
    /// still drives the target percentage text.
    pub shr_frac: f32,
    /// Handoff flash intensity: 1.0 = just flashed, 0.0 = none.
    pub flash_t: f32,
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

/// One bar segment's content: its percentage, label, and filled-cell colour.
/// `fill` is chosen per segment by the caller (e.g. `fill_color(frac)` for a
/// warn-at-full bar like `ctx`, a flat neutral colour for a bar where 100%
/// isn't a warning, like `shr`).
struct Seg<'a> {
    pct: Option<u8>,
    /// Eased fill fraction (0.0..1.0) — drives the bar, independent of
    /// `pct`'s target text (they can differ mid-ease).
    frac: f32,
    label: &'a str,
    fill: (u8, u8, u8),
}

/// The left-eighth block for a fractional cell (0.0..1.0): ▏(1/8) … ▉(7/8).
/// None below 1/8 — an empty track cell reads cleaner than a sliver.
pub(crate) fn partial_block(frac_cells: f32) -> Option<char> {
    let eighths = (frac_cells.clamp(0.0, 1.0) * 8.0).floor() as u32;
    match eighths.min(7) {
        0 => None,
        // U+2589 ▉ is 7/8 … U+258F ▏ is 1/8: codepoint = 0x2590 - eighths.
        n => char::from_u32(0x2590 - n),
    }
}

/// One `<bar> NN% (label)` segment. `seg.pct = None` draws an all-track bar
/// and ` 0%`.
fn push_segment(
    cells: &mut Vec<CellView>,
    row: u16,
    col: u16,
    max_col: u16,
    seg: Seg,
    pal: &Pal,
) -> u16 {
    let Seg {
        pct,
        frac,
        label,
        fill,
    } = seg;
    let bg = crew_theme::theme().page_bg;
    let filled_cells = frac * BAR_W as f32;
    let full = filled_cells.floor() as usize;
    let partial = partial_block(filled_cells.fract());
    let mut x = col;
    for i in 0..BAR_W {
        if x >= max_col {
            break;
        }
        let (ch, fg) = if i < full {
            ('\u{2588}', fill)
        } else if i == full && partial.is_some() {
            (partial.unwrap(), fill)
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
    now: u64,
) -> Vec<CellView> {
    let t = crew_theme::theme();
    let pal = Pal {
        ink: t.ink,
        dim: t.text_muted,
    };
    let mut cells = Vec::new();
    for (i, v) in views.iter().take(lay.shown).enumerate() {
        let row = start_row + i as u16;
        // Working agent breathes toward the accent (≤25% blend, 1600ms
        // triangle); a fresh handoff flashes the row (≤35%, 400ms fade).
        // Flash wins over pulse when both apply — it's the newer signal.
        let base = agent_color(&v.name);
        let color = if v.flash_t > 0.0 {
            crate::anim::lerp_rgb(base, crate::palette::accent(), 0.35 * v.flash_t)
        } else if v.active {
            crate::anim::lerp_rgb(
                base,
                crate::palette::accent(),
                0.25 * crate::anim::tri(now, 1600),
            )
        } else {
            base
        };
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
            let seg = Seg {
                pct: v.ctx_pct,
                frac: v.ctx_frac,
                label: "ctx",
                fill: fill_color(v.ctx_frac),
            };
            col = push_segment(&mut cells, row, col, cols, seg, &pal);
        }
        if lay.level == 0 {
            col = place(&mut cells, row, col, cols, SEP, pal.dim, false);
            // Share is relative, not a fullness warning — a lone active
            // agent is always 100% and that isn't alarming — so it stays a
            // neutral accent rather than riding the ctx bar's red-at-full
            // scale.
            let seg = Seg {
                pct: v.share_pct,
                frac: v.shr_frac,
                label: "shr",
                fill: crate::palette::accent(),
            };
            col = push_segment(&mut cells, row, col, cols, seg, &pal);
        }
        let _ = col;
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_pal() -> Pal {
        Pal {
            ink: (255, 255, 255),
            dim: (128, 128, 128),
        }
    }

    /// Chars in `row`, sorted by column — mirrors the `text_at` pattern in
    /// `row_cells_have_name_state_pipes_and_ctx_shr_bars`.
    fn row_text(cells: &[CellView], row: u16) -> String {
        let mut row_cells: Vec<(u16, char)> = cells
            .iter()
            .filter(|c| c.row == row)
            .map(|c| (c.col, c.c))
            .collect();
        row_cells.sort();
        row_cells.into_iter().map(|(_, c)| c).collect()
    }

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
            ctx_frac: ctx.map(|p| p as f32 / 100.0).unwrap_or(0.0),
            shr_frac: share.map(|p| p as f32 / 100.0).unwrap_or(0.0),
            flash_t: 0.0,
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
        let cells = row_cells(&views, cols, start_row, &lay, 0);

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

    #[test]
    fn partial_block_selects_left_eighths() {
        assert_eq!(partial_block(0.0), None);
        assert_eq!(partial_block(0.05), None, "below 1/8 draws nothing");
        assert_eq!(partial_block(0.125), Some('\u{258F}'), "1/8 \u{258F}");
        assert_eq!(partial_block(0.5), Some('\u{258C}'), "4/8 \u{258C}");
        assert_eq!(partial_block(0.874), Some('\u{258A}'), "6/8 \u{258A}");
        assert_eq!(
            partial_block(0.999),
            Some('\u{2589}'),
            "caps at 7/8 \u{2589}"
        );
    }

    #[test]
    fn segment_bar_uses_eased_frac_but_target_pct_text() {
        // frac mid-ease (0.35 of BAR_W=6 → 2 full cells + 0.1 partial → none),
        // while the pct text must read the target (50%).
        let mut cells = Vec::new();
        let pal = test_pal();
        let seg = Seg {
            pct: Some(50),
            frac: 0.35,
            label: "ctx",
            fill: (255, 0, 0),
        };
        push_segment(&mut cells, 0, 0, 40, seg, &pal);
        let row: String = row_text(&cells, 0); // existing test helper pattern
        assert!(row.contains("50%"), "text shows target: {row}");
        let full = row.chars().filter(|&c| c == '\u{2588}').count();
        assert_eq!(full, 2, "0.35 * 6 = 2.1 cells → 2 full blocks: {row}");
    }

    /// Build one AgentView with given active/flash_t, run layout + row_cells,
    /// and return the fg color of the first cell (the marker).
    fn name_fg(active: bool, flash_t: f32, now: u64) -> (u8, u8, u8) {
        let mut view = v("test", "idle", 0, None, None, active);
        view.flash_t = flash_t;
        let views = vec![view];
        let lay = layout(&views, 80, 1).expect("test layout");
        let cells = row_cells(&views, 80, 0, &lay, now);
        // Find the first cell (should be the marker at row 0, col 0).
        cells
            .iter()
            .find(|c| c.row == 0)
            .map(|c| c.fg)
            .expect("marker cell found")
    }

    #[test]
    fn active_agent_pulses_toward_accent() {
        // At tri peak (now = period/2 = 800) an active row's name color must
        // differ from an idle row's; at amplitude ≤ 0.25 it must not reach
        // the accent itself.
        let accent = crate::palette::accent();
        let quiet = name_fg(false, 0.0, 0);
        let peak = name_fg(true, 0.0, 800);
        assert_ne!(quiet, peak, "active row breathes");
        assert_ne!(peak, accent, "amplitude stays subtle");
        let trough = name_fg(true, 0.0, 0);
        assert_eq!(trough, quiet, "tri trough = base color");
    }

    #[test]
    fn handoff_flash_blends_and_expires() {
        let fresh = name_fg(false, 1.0, 0);
        let gone = name_fg(false, 0.0, 0);
        assert_ne!(fresh, gone, "fresh flash tints the row");
    }

    #[test]
    fn flash_overrides_pulse_when_both_apply() {
        // At tri peak (now=800) an active agent with a fresh flash must render
        // exactly like an inactive one with the same flash — flash wins.
        assert_eq!(name_fg(true, 1.0, 800), name_fg(false, 1.0, 800));
    }
}
