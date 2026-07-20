//! The crew pane's empty state: instead of a bare one-line hint, a fresh pane
//! introduces the crew — connection state, the detected agents with their
//! roles, and how to start (plain task, or `@agent` to pick who begins) — so
//! the first run explains itself.
use crew_plugin::AgentInfo;
use crew_render::CellView;

/// A text row at `(row, col..)`, clipped to `cols`.
fn line(
    out: &mut Vec<CellView>,
    row: u16,
    col: u16,
    cols: u16,
    s: &str,
    fg: (u8, u8, u8),
    bold: bool,
) {
    let bg = crew_theme::theme().page_bg;
    // Width-aware (see `chatwidth`): roster names can carry wide glyphs.
    crate::chatwidth::place_row(col, cols, s.chars().map(|c| (c, fg)), |x, c, fg| {
        out.push(CellView {
            col: x,
            row,
            c,
            fg,
            bg,
            bold,
            italic: false,
        });
    });
}

/// Emit one onboarding row (skipped below `max_row`) and advance the cursor.
fn put(
    cells: &mut Vec<CellView>,
    row: &mut u16,
    max_row: u16,
    cols: u16,
    s: &str,
    fg: (u8, u8, u8),
    bold: bool,
) {
    if *row < max_row {
        line(cells, *row, 1, cols, s, fg, bold);
    }
    *row += 1;
}

/// Render the onboarding block into rows `top..max_row`.
pub(crate) fn empty_cells(
    cols: u16,
    max_row: u16,
    top: u16,
    connected: bool,
    agents: &[AgentInfo],
) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut cells = Vec::new();
    let mut row = top + 1;
    if !connected {
        put(
            &mut cells,
            &mut row,
            max_row,
            cols,
            "\u{25cb} connecting to the crew broker\u{2026}",
            t.text_muted,
            false,
        );
        return cells;
    }
    if agents.is_empty() {
        put(
            &mut cells,
            &mut row,
            max_row,
            cols,
            "No agents available.",
            t.ink,
            true,
        );
        put(&mut cells, &mut row, max_row, cols, "", t.text_muted, false);
        put(
            &mut cells,
            &mut row,
            max_row,
            cols,
            "Set OPENROUTER_API_KEY, DASHSCOPE_API_KEY, or",
            t.text_muted,
            false,
        );
        put(
            &mut cells,
            &mut row,
            max_row,
            cols,
            "ANTHROPIC_API_KEY, then reopen /smith.",
            t.text_muted,
            false,
        );
    } else {
        // Minimal, Claude-Code-style: a single muted hint. No roster dump and
        // no keybind table — the pane shouldn't spend rows on chrome before the
        // first task. `@agent` picks who starts; plain text runs the swarm.
        let first = &agents[0].name;
        put(
            &mut cells,
            &mut row,
            max_row,
            cols,
            &format!("Type a task and press Enter \u{2014} @agent to pick who starts (e.g. @{first}), / for commands."),
            t.text_muted,
            false,
        );
    }
    cells
}

#[cfg(test)]
#[path = "chatempty_tests.rs"]
mod tests;
