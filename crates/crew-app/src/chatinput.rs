//! The crew pane's input composer. Tall panes get two rows: an affordance bar
//! naming the addressable `@agents` (each in its roster colour) with key hints
//! right-aligned, then a `❯` prompt row that highlights a valid leading
//! `@agent` mention in that agent's colour. Short panes fall back to the
//! single prompt row.
use crew_plugin::AgentInfo;
use crew_render::CellView;

/// Rows the composer occupies at this pane height (the affordance bar needs
/// some vertical room to be worth its row).
pub(crate) fn composer_rows(rows: u16) -> u16 {
    if rows >= 7 {
        2
    } else {
        1
    }
}

fn cell(col: u16, row: u16, c: char, fg: (u8, u8, u8), bold: bool) -> CellView {
    CellView {
        col,
        row,
        c,
        fg,
        bg: crew_theme::theme().page_bg,
        bold,
        italic: false,
    }
}

/// Chars of the leading `@agent` mention when it names a known agent
/// (`@coder fix this` → 6), else 0.
fn mention_len(input: &str, agents: &[AgentInfo]) -> usize {
    let Some(rest) = input.strip_prefix('@') else {
        return 0;
    };
    let name = rest.split_whitespace().next().unwrap_or("");
    if agents.iter().any(|a| a.name.eq_ignore_ascii_case(name)) {
        1 + name.len()
    } else {
        0
    }
}

/// The `❯ input▏` prompt line at `row`, with a valid `@mention` coloured.
fn prompt_cells(input: &str, agents: &[AgentInfo], cols: u16, row: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let mention = mention_len(input, agents);
    let m_color = if mention > 0 {
        crate::chatroster::agent_color(&input[1..mention])
    } else {
        t.ink
    };
    let mut cells = vec![cell(0, row, '\u{276f}', accent, true)]; // ❯
    let styled = input
        .chars()
        .enumerate()
        .map(|(i, c)| (c, (if i < mention { m_color } else { t.ink }, i < mention)));
    // Width-aware placement: a wide glyph advances two columns, and the caret
    // lands after the text instead of on top of it.
    let x = crate::chatwidth::place_row(2, cols, styled, |x, c, (fg, bold)| {
        cells.push(cell(x, row, c, fg, bold));
    });
    if x < cols {
        cells.push(cell(x, row, '\u{258f}', accent, false)); // ▏ caret
    }
    cells
}

/// The muted affordance bar: `@agent` chips left, key hints right.
fn bar_cells(agents: &[AgentInfo], cols: u16, row: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut cells = Vec::new();
    let mut x = 0u16;
    for a in agents {
        let chip = format!("@{}", a.name);
        if x + chip.len() as u16 + 2 > cols {
            break;
        }
        let fg = crate::chatroster::agent_color(&a.name);
        for c in chip.chars() {
            cells.push(cell(x, row, c, fg, false));
            x += 1;
        }
        x += 2;
    }
    let hints = "Tab complete \u{00b7} /help \u{00b7} Enter send \u{00b7} Esc close";
    let hw = hints.chars().count() as u16;
    if cols > hw && cols - hw > x {
        for (hx, c) in (cols - hw..).zip(hints.chars()) {
            cells.push(cell(hx, row, c, t.text_muted, false));
        }
    }
    cells
}

/// Render the composer into the bottom `composer_rows(rows)` rows.
pub(crate) fn composer_cells(
    input: &str,
    agents: &[AgentInfo],
    cols: u16,
    rows: u16,
) -> Vec<CellView> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    let mut cells = Vec::new();
    if composer_rows(rows) == 2 {
        cells.extend(bar_cells(agents, cols, rows - 2));
    }
    cells.extend(prompt_cells(input, agents, cols, rows - 1));
    cells
}

#[cfg(test)]
#[path = "chatinput_tests.rs"]
mod tests;

/// Pure input reducer.
///
/// - `enter`: return `Some(old_input)`, clear `input`.
/// - `backspace`: pop last char, return `None`.
/// - `ch=Some(c)` (non-control): push `c`, return `None`.
pub fn input_reduce(
    input: &mut String,
    ch: Option<char>,
    enter: bool,
    backspace: bool,
) -> Option<String> {
    if enter {
        Some(std::mem::take(input))
    } else if backspace {
        input.pop();
        None
    } else if let Some(c) = ch {
        if !c.is_control() {
            input.push(c);
        }
        None
    } else {
        None
    }
}
