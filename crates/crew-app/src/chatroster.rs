//! The crew pane's agent roster row: one chip per agent — a colored dot + name
//! in a stable per-agent colour, followed by a dimmed model badge — so it's
//! always visible which agents are on the crew and which model each one runs.
use crew_plugin::AgentInfo;
use crew_render::CellView;

/// Stable colour for an agent name: a small hash picks from the theme's bright
/// ANSI palette (skipping black/white), so `planner` renders the same colour
/// every frame and across panes, and agents are told apart at a glance.
pub(crate) fn agent_color(name: &str) -> (u8, u8, u8) {
    // Bright red..bright cyan (ANSI 9..=14): distinct, readable on the page bg.
    let palette = &crew_theme::theme().ansi[9..=14];
    let h = name.bytes().fold(0xcbf2_9ce4u32, |h, b| {
        (h ^ b as u32).wrapping_mul(0x0100_0193)
    });
    palette[(h as usize) % palette.len()]
}

/// A compact model badge: the slug's last path segment, minus a `:free`-style
/// variant suffix — `meta-llama/llama-3.3-70b-instruct:free` → `llama-3.3-70b-instruct`.
pub(crate) fn short_model(model: &str) -> &str {
    let tail = model.rsplit('/').next().unwrap_or(model);
    tail.split(':').next().unwrap_or(tail)
}

/// Append `s` at `(row, col..)` in `fg`, clipped to `cols`; returns the next column.
fn push(
    cells: &mut Vec<CellView>,
    row: u16,
    col: u16,
    cols: u16,
    s: &str,
    fg: (u8, u8, u8),
    bold: bool,
) -> u16 {
    let bg = crew_theme::theme().page_bg;
    // Width-aware placement (wide glyphs advance two columns — see `chatwidth`).
    crate::chatwidth::place_row(col, cols, s.chars().map(|c| (c, fg)), |x, c, fg| {
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

/// An agent's chip stat suffix from its `(replies, total ms)` totals:
/// `·3× 4.2s` (reply count and average latency). Empty until it has replied.
pub(crate) fn chip_stat(
    stats: &std::collections::HashMap<String, (u32, u64)>,
    name: &str,
) -> String {
    match stats.get(name) {
        Some((n, ms)) if *n > 0 => {
            let avg = *ms as f32 / (*n as f32 * 1000.0);
            format!("\u{00b7}{n}\u{00d7} {avg:.1}s")
        }
        _ => String::new(),
    }
}

/// Build the roster row at `row`: `▪ name model ·n× avg` chips, two spaces
/// apart, clipped to the pane width. The `active` agent's chip gets a `▸`
/// marker and a bold name so the currently-thinking agent stands out; the
/// dimmed stat suffix comes from `stats` (per-agent replies + total ms).
/// Empty when no agents are known.
pub(crate) fn roster_cells(
    cols: u16,
    row: u16,
    agents: &[AgentInfo],
    active: &[&str],
    stats: &std::collections::HashMap<String, (u32, u64)>,
) -> Vec<CellView> {
    let mut cells = Vec::new();
    let muted = crew_theme::theme().text_muted;
    let mut x = 0u16;
    for (i, a) in agents.iter().enumerate() {
        if i > 0 {
            x += 2; // gap between chips
        }
        if x >= cols {
            break;
        }
        let c = agent_color(&a.name);
        let is_active = active.iter().any(|n| n.eq_ignore_ascii_case(&a.name));
        let marker = if is_active { "\u{25b8} " } else { "\u{25aa} " }; // ▸ / ▪
        x = push(&mut cells, row, x, cols, marker, c, is_active);
        x = push(&mut cells, row, x, cols, &a.name, c, is_active);
        let m = short_model(&a.model);
        if !m.is_empty() {
            x = push(&mut cells, row, x, cols, " ", muted, false);
            x = push(&mut cells, row, x, cols, m, muted, false);
        }
        let stat = chip_stat(stats, &a.name);
        if !stat.is_empty() {
            x = push(&mut cells, row, x, cols, " ", muted, false);
            x = push(&mut cells, row, x, cols, &stat, muted, false);
        }
    }
    cells
}

#[cfg(test)]
#[path = "chatroster_tests.rs"]
mod tests;
