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
    for (i, c) in s.chars().enumerate() {
        let x = col + i as u16;
        if x >= cols {
            break;
        }
        out.push(CellView {
            col: x,
            row,
            c,
            fg,
            bg,
            bold,
            italic: false,
        });
    }
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
            "Set OPENROUTER_API_KEY or ANTHROPIC_API_KEY",
            t.text_muted,
            false,
        );
        put(
            &mut cells,
            &mut row,
            max_row,
            cols,
            "and reopen /crew.",
            t.text_muted,
            false,
        );
        return cells;
    }
    put(
        &mut cells,
        &mut row,
        max_row,
        cols,
        "Your crew is ready.",
        t.ink,
        true,
    );
    put(&mut cells, &mut row, max_row, cols, "", t.text_muted, false);
    for a in agents {
        if row >= max_row {
            break;
        }
        let color = crate::chatroster::agent_color(&a.name);
        let name = format!("\u{25aa} {}", a.name);
        line(&mut cells, row, 1, cols, &name, color, true);
        if !a.role.is_empty() {
            let at = 1 + name.chars().count() as u16;
            line(
                &mut cells,
                row,
                at,
                cols,
                &format!(" \u{2014} {}", a.role),
                t.text_muted,
                false,
            );
        }
        row += 1;
    }
    put(&mut cells, &mut row, max_row, cols, "", t.text_muted, false);
    put(
        &mut cells,
        &mut row,
        max_row,
        cols,
        "Type a task and press Enter \u{2014} the crew relays it until @done.",
        t.text_muted,
        false,
    );
    let first = agents[0].name.clone();
    put(
        &mut cells,
        &mut row,
        max_row,
        cols,
        &format!("Try: @{first} plan a small refactor and hand it to the crew"),
        t.hint_fg,
        false,
    );
    cells
}

#[cfg(test)]
#[path = "chatempty_tests.rs"]
mod tests;
