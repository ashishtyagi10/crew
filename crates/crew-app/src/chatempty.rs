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
            "ANTHROPIC_API_KEY, then reopen /crew.",
            t.text_muted,
            false,
        );
    } else {
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
    }
    append_quick_start(&mut cells, row, max_row, cols);
    cells
}

/// One quick-start row: `(key, primary clause, optional secondary clause)`.
/// Fixed copy from the design spec, kept as a data table (rather than
/// pre-joined strings) so the width rule below — drop secondary clauses,
/// then drop the whole block — measures structured lengths instead of
/// slicing rendered text.
const QUICK_START: &[(&str, &str, Option<&str>)] = &[
    ("Enter", "send", Some("type while busy to queue")),
    (
        "@agent",
        "address one agent",
        Some("plain text runs a swarm"),
    ),
    ("Esc", "interrupt a running turn (idle: close pane)", None),
    (
        "Ctrl+O",
        "compact transcript",
        Some("Ctrl+Shift+M raw text"),
    ),
    ("/", "command palette", None),
];

/// Column the quick-start keys start at — matches the rest of the block's
/// left indent (`put`/`line` above both use `col = 1`).
const QUICK_START_COL: u16 = 1;
/// Gap between the widest key and the aligned description column.
const QUICK_START_GAP: u16 = 3;

/// Column the descriptions start at: aligned past the widest key.
fn quick_start_desc_col() -> u16 {
    let max_key = QUICK_START
        .iter()
        .map(|(k, _, _)| k.chars().count() as u16)
        .max()
        .unwrap_or(0);
    QUICK_START_COL + max_key + QUICK_START_GAP
}

/// Display width of one row's description, with or without its secondary
/// clause (` \u{b7} ` is 3 columns).
fn quick_start_line_len(primary: &str, secondary: Option<&str>, with_clause: bool) -> u16 {
    let mut n = primary.chars().count() as u16;
    if with_clause {
        if let Some(s) = secondary {
            n += 3 + s.chars().count() as u16;
        }
    }
    n
}

/// Append the "quick start" keybind hints below the existing empty-state
/// content, at `row` (the next free row after whatever `connected`/`agents`
/// branch drew). Height-gated (dropped whole on a short pane) then
/// width-gated (secondary ` \u{b7} ` clauses drop first, then the whole
/// block — never wrapped mid-hint).
fn append_quick_start(cells: &mut Vec<CellView>, row: u16, max_row: u16, cols: u16) {
    // One blank separator row, the `─ quick start ─` rule, then one row per hint.
    let needed_rows = 2 + QUICK_START.len() as u16;
    if max_row < row.saturating_add(needed_rows) {
        return;
    }
    let desc_col = quick_start_desc_col();
    let full_w = desc_col
        + QUICK_START
            .iter()
            .map(|(_, p, s)| quick_start_line_len(p, *s, true))
            .max()
            .unwrap_or(0);
    let bare_w = desc_col
        + QUICK_START
            .iter()
            .map(|(_, p, s)| quick_start_line_len(p, *s, false))
            .max()
            .unwrap_or(0);
    if cols < bare_w {
        return;
    }
    let with_clauses = cols >= full_w;
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let rule_row = row + 1;
    let rule = crate::boxdraw::section_header(
        "quick start",
        cols,
        t.border_normal,
        t.legend_off,
        t.page_bg,
    );
    cells.extend(rule.into_iter().map(|mut c| {
        c.row = rule_row;
        c
    }));
    for (i, (key, primary, secondary)) in QUICK_START.iter().enumerate() {
        let r = rule_row + 1 + i as u16;
        line(cells, r, QUICK_START_COL, cols, key, accent, false);
        let mut desc = (*primary).to_string();
        if with_clauses {
            if let Some(s) = secondary {
                desc.push_str(" \u{b7} ");
                desc.push_str(s);
            }
        }
        line(cells, r, desc_col, cols, &desc, t.text_muted, false);
    }
}

#[cfg(test)]
#[path = "chatempty_tests.rs"]
mod tests;
