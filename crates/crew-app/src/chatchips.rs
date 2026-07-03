//! The crew pane's dense agent chip grid: one compact colored cluster per
//! agent (`▸planner qwen-max ⠙3.2s 4.1k 38% 42%`), packed equal-width and
//! wrapped to fill the pane. Replaces the per-agent pulse lanes. Pure text +
//! geometry; `chatview` turns the chosen layout into cells.
use crew_render::CellView;

use crate::chathdr::fmt_tokens;
use crate::chatroster::agent_color;

/// A 2-space gutter between clusters on a row.
const GUTTER: usize = 2;
/// Sparsest drop level (marker+name+state only).
const MAX_LEVEL: u8 = 4;

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

/// (text, colour, bold) runs for one cluster at `level` (0 = richest).
/// Fields shed from the right as `level` rises: share%, ctx%, tok, model.
pub(crate) fn cluster_runs(v: &AgentView, level: u8) -> Vec<(String, (u8, u8, u8), bool)> {
    let t = crew_theme::theme();
    let color = agent_color(&v.name);
    let mut runs: Vec<(String, (u8, u8, u8), bool)> = Vec::new();
    let marker = if v.active { "\u{25b8}" } else { "\u{25aa}" }; // ▸ / ▪
    runs.push((format!("{marker}{}", v.name), color, v.active));
    if level < 4 && !v.model.is_empty() {
        runs.push((format!(" {}", v.model), t.text_muted, false));
    }
    runs.push((format!(" {}", v.state), t.text_muted, false));
    if level < 3 && v.tok > 0 {
        runs.push((format!(" {}", fmt_tokens(v.tok)), t.text_muted, false));
    }
    if level < 2 {
        if let Some(p) = v.ctx_pct {
            runs.push((format!(" {p}%"), t.text_muted, false));
        }
    }
    if level < 1 {
        if let Some(p) = v.share_pct {
            runs.push((format!(" {p}%"), t.text_muted, false));
        }
    }
    runs
}

/// Rendered width (display columns) of a cluster at `level`.
pub(crate) fn cluster_width(v: &AgentView, level: u8) -> usize {
    cluster_runs(v, level)
        .iter()
        .map(|(s, _, _)| crate::chatwidth::str_w(s))
        .sum()
}

/// The widest cluster across `views` at `level`.
fn max_width(views: &[AgentView], level: u8) -> usize {
    views
        .iter()
        .map(|v| cluster_width(v, level))
        .max()
        .unwrap_or(0)
}

/// The richest level (0 best) whose widest cluster fits `cols`; `None` if even
/// the sparsest cluster overflows.
pub(crate) fn choose_level(views: &[AgentView], cols: u16) -> Option<u8> {
    if views.is_empty() {
        return None;
    }
    for level in 0..=MAX_LEVEL {
        if max_width(views, level) <= cols as usize {
            return Some(level);
        }
    }
    None
}

/// Clusters that fit on one row of `cols`, given equal `cluster_w` + gutter.
pub(crate) fn per_row(cluster_w: usize, cols: u16) -> usize {
    let cols = cols as usize;
    if cluster_w == 0 || cluster_w > cols {
        return 1;
    }
    // n clusters need n*w + (n-1)*gutter columns.
    ((cols + GUTTER) / (cluster_w + GUTTER)).max(1)
}

/// Rows the grid needs for `views` at `cols` (0 when nothing fits).
pub(crate) fn grid_rows(views: &[AgentView], cols: u16) -> u16 {
    let Some(level) = choose_level(views, cols) else {
        return 0;
    };
    let w = max_width(views, level);
    let cols_per = per_row(w, cols);
    views.len().div_ceil(cols_per) as u16
}

/// Place the chip grid starting at `start_row`, filling `cols`. Clusters are
/// padded to the chosen level's max width and separated by the gutter, wrapping
/// to new rows. Returns the cells (empty when nothing fits).
pub(crate) fn grid_cells(views: &[AgentView], cols: u16, start_row: u16) -> Vec<CellView> {
    let Some(level) = choose_level(views, cols) else {
        return Vec::new();
    };
    let w = max_width(views, level);
    let cols_per = per_row(w, cols);
    let t = crew_theme::theme();
    let mut cells = Vec::new();
    for (i, v) in views.iter().enumerate() {
        let row = start_row + (i / cols_per) as u16;
        let col0 = (i % cols_per) * (w + GUTTER);
        let mut x = col0 as u16;
        let max_col = (col0 + w) as u16;
        for (s, color, bold) in cluster_runs(v, level) {
            x = crate::chatwidth::place_row(
                x,
                max_col,
                s.chars().map(|c| (c, color)),
                |px, c, fg| {
                    cells.push(CellView {
                        col: px,
                        row,
                        c,
                        fg,
                        bg: t.page_bg,
                        bold,
                        italic: false,
                    });
                },
            );
        }
    }
    cells
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
    fn cluster_runs_full_level_has_all_fields_in_order() {
        let runs = cluster_runs(&v("planner", true), 0);
        let joined: String = runs.iter().map(|(s, _, _)| s.as_str()).collect();
        assert!(
            joined.starts_with("\u{25b8}planner"),
            "marker+name: {joined}"
        );
        assert!(joined.contains("qwen-max"), "model: {joined}");
        assert!(joined.contains("\u{2819}3.2s"), "state: {joined}");
        assert!(joined.contains("4.1k"), "tok: {joined}");
        assert!(joined.contains("38%"), "ctx: {joined}");
        assert!(joined.contains("42%"), "share: {joined}");
        // Name run is the agent colour and bold while active.
        assert!(runs
            .iter()
            .any(|(s, _, bold)| s.contains("planner") && *bold));
    }

    #[test]
    fn drop_levels_shed_fields_from_the_right() {
        let a = v("planner", false);
        let full = cluster_runs(&a, 0)
            .iter()
            .map(|(s, _, _)| s.clone())
            .collect::<String>();
        assert!(full.contains("42%") && full.contains("38%") && full.contains("4.1k"));
        let l1: String = cluster_runs(&a, 1)
            .iter()
            .map(|(s, _, _)| s.clone())
            .collect();
        assert!(
            !l1.contains("42%") && l1.contains("38%"),
            "L1 drops share: {l1}"
        );
        let l2: String = cluster_runs(&a, 2)
            .iter()
            .map(|(s, _, _)| s.clone())
            .collect();
        assert!(
            !l2.contains("38%") && l2.contains("4.1k"),
            "L2 drops ctx: {l2}"
        );
        let l3: String = cluster_runs(&a, 3)
            .iter()
            .map(|(s, _, _)| s.clone())
            .collect();
        assert!(
            !l3.contains("4.1k") && l3.contains("qwen-max"),
            "L3 drops tok: {l3}"
        );
        let l4: String = cluster_runs(&a, 4)
            .iter()
            .map(|(s, _, _)| s.clone())
            .collect();
        assert!(
            !l4.contains("qwen-max") && l4.contains("planner"),
            "L4 drops model: {l4}"
        );
        assert!(l4.contains("2\u{00d7}"), "L4 keeps state: {l4}");
    }

    #[test]
    fn choose_level_prefers_richer_when_wide_and_degrades_when_narrow() {
        let views = vec![v("planner", false), v("coder", false)];
        assert_eq!(choose_level(&views, 200), Some(0), "wide → full");
        let l = choose_level(&views, 22).expect("still fits a minimal cluster");
        assert!(l >= 3, "narrow → a sparse level, got {l}");
        assert_eq!(choose_level(&views, 3), None, "too narrow for any cluster");
    }

    #[test]
    fn grid_rows_packs_multiple_per_row_and_wraps() {
        let views = vec![v("a", false), v("b", false), v("c", false)];
        // Very wide: all three on one row.
        assert_eq!(grid_rows(&views, 240), 1);
        // Enough for one cluster per row: three rows.
        let w = cluster_width(&views[0], choose_level(&views, 26).unwrap()) as u16;
        assert_eq!(grid_rows(&views, w + 2), 3);
        // Nothing fits.
        assert_eq!(grid_rows(&views, 3), 0);
    }

    #[test]
    fn per_row_is_at_least_one_and_accounts_for_the_gutter() {
        assert_eq!(per_row(10, 10), 1);
        assert_eq!(per_row(10, 21), 1); // 10 + 2 gutter + 10 = 22 > 21
        assert_eq!(per_row(10, 22), 2);
    }
}
