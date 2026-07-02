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

/// Build the roster row at `row`: `▪ name model` chips, two spaces apart,
/// clipped to the pane width. The `active` agent's chip gets a `▸` marker and a
/// bold name so the currently-thinking agent stands out. Empty when no agents
/// are known.
pub(crate) fn roster_cells(
    cols: u16,
    row: u16,
    agents: &[AgentInfo],
    active: &[&str],
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
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(name: &str, model: &str) -> AgentInfo {
        AgentInfo {
            name: name.into(),
            role: String::new(),
            model: model.into(),
        }
    }

    fn text(cells: &[CellView]) -> String {
        let mut v: Vec<(u16, char)> = cells.iter().map(|c| (c.col, c.c)).collect();
        v.sort_unstable();
        v.into_iter().map(|(_, c)| c).collect()
    }

    #[test]
    fn short_model_strips_provider_and_variant() {
        assert_eq!(
            short_model("meta-llama/llama-3.3-70b-instruct:free"),
            "llama-3.3-70b-instruct"
        );
        assert_eq!(short_model("claude-sonnet-5"), "claude-sonnet-5");
        assert_eq!(short_model(""), "");
    }

    #[test]
    fn agent_color_is_stable_and_distinguishes_names() {
        assert_eq!(agent_color("planner"), agent_color("planner"));
    }

    #[test]
    fn roster_row_lists_agents_with_model_badges() {
        let agents = [agent("planner", "org/m-1:free"), agent("coder", "")];
        let line = text(&roster_cells(80, 1, &agents, &[]));
        assert!(line.contains("planner m-1"), "got: {line}");
        assert!(line.contains("coder"), "got: {line}");
    }

    #[test]
    fn active_agent_gets_arrow_marker_and_bold_name() {
        let agents = [agent("planner", ""), agent("coder", "")];
        let cells = roster_cells(80, 1, &agents, &["coder"]);
        let line = text(&cells);
        assert!(line.contains("\u{25b8} coder"), "got: {line}");
        assert!(line.contains("\u{25aa} planner"), "got: {line}");
        assert!(cells.iter().any(|c| c.bold), "active chip should be bold");
    }

    #[test]
    fn several_active_agents_highlight_together() {
        let agents = [
            agent("planner", ""),
            agent("coder", ""),
            agent("reviewer", ""),
        ];
        let line = text(&roster_cells(80, 1, &agents, &["planner", "coder"]));
        assert!(line.contains("\u{25b8} planner"), "got: {line}");
        assert!(line.contains("\u{25b8} coder"), "got: {line}");
        assert!(line.contains("\u{25aa} reviewer"), "got: {line}");
    }

    #[test]
    fn roster_clips_to_width_and_targets_row() {
        let agents = [agent("a-very-long-agent-name", "some/very-long-model")];
        let cells = roster_cells(10, 1, &agents, &[]);
        assert!(cells.iter().all(|c| c.col < 10 && c.row == 1));
    }

    #[test]
    fn empty_roster_renders_nothing() {
        assert!(roster_cells(80, 1, &[], &[]).is_empty());
    }
}
