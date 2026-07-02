//! The crew pane's input composer. Tall panes get a bordered fieldset card:
//! the addressable `@agents` ride the top border as the legend (each in its
//! roster colour), the `❯` prompt sits on the interior row (a valid leading
//! `@agent` mention takes that agent's colour), and the key hints ride the
//! bottom border. Short panes fall back to a single bare prompt row.
use crew_plugin::AgentInfo;
use crew_render::CellView;

/// Rows the composer occupies at this pane height (the bordered card needs
/// top border + prompt + bottom border to be worth the room).
pub(crate) fn composer_rows(rows: u16) -> u16 {
    if rows >= 7 {
        3
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

/// The `❯ input▏` prompt line at `row`, starting at column `x0` and clipped to
/// `[x0, max)`, with a valid `@mention` coloured.
fn prompt_cells(input: &str, agents: &[AgentInfo], x0: u16, max: u16, row: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let mention = mention_len(input, agents);
    let m_color = if mention > 0 {
        crate::chatroster::agent_color(&input[1..mention])
    } else {
        t.ink
    };
    let mut cells = vec![cell(x0, row, '\u{276f}', accent, true)]; // ❯
                                                                   // Mid-message @file mentions read as chips: tinted accent while typed.
    let spans = crate::chatmention::spans(input);
    let in_span = |i: usize| spans.iter().any(|&(s, e)| i >= s && i < e);
    let styled = input.chars().enumerate().map(|(i, c)| {
        let (fg, bold) = if i < mention {
            (m_color, true)
        } else if in_span(i) {
            (accent, false)
        } else {
            (t.ink, false)
        };
        (c, (fg, bold))
    });
    // Width-aware placement: a wide glyph advances two columns, and the caret
    // lands after the text instead of on top of it.
    let x = crate::chatwidth::place_row(x0 + 2, max, styled, |x, c, (fg, bold)| {
        cells.push(cell(x, row, c, fg, bold));
    });
    if x < max {
        cells.push(cell(x, row, '\u{258f}', accent, false)); // ▏ caret
    }
    cells
}

/// Overlay the `@agent` chips on the card's top border as its legend
/// (`╭─ @planner @coder ─╮`), each chip in its roster colour.
fn chips_on_border(cells: &mut Vec<CellView>, agents: &[AgentInfo], cols: u16, row: u16) {
    let bg = crew_theme::theme().page_bg;
    let border = crew_theme::theme().border_normal;
    let mut x = 2u16;
    for a in agents {
        let chip = format!("@{}", a.name);
        // Chip + its two surrounding spaces must stay clear of the corner.
        if x + chip.len() as u16 + 2 > cols.saturating_sub(2) {
            break;
        }
        let fg = crate::chatroster::agent_color(&a.name);
        cells.push(CellView {
            col: x,
            row,
            c: ' ',
            fg: border,
            bg,
            bold: false,
            italic: false,
        });
        x += 1;
        for c in chip.chars() {
            cells.push(cell(x, row, c, fg, false));
            x += 1;
        }
    }
    if x > 2 {
        cells.push(CellView {
            col: x,
            row,
            c: ' ',
            fg: border,
            bg,
            bold: false,
            italic: false,
        });
    }
}

/// Right-align the key hints on the card's bottom border (muted, framed in
/// spaces so the rule reads as a fieldset edge).
fn hints_on_border(cells: &mut Vec<CellView>, cols: u16, row: u16) {
    let t = crew_theme::theme();
    let hints = "Tab complete \u{00b7} /help \u{00b7} Enter send \u{00b7} Esc close";
    let label = format!(" {hints} ");
    let w = label.chars().count() as u16;
    if w + 3 >= cols {
        return;
    }
    for (hx, c) in (cols - 2 - w..).zip(label.chars()) {
        cells.push(cell(hx, row, c, t.text_muted, false));
    }
}

/// Render the composer into the bottom `composer_rows(rows)` rows: a bordered
/// fieldset card on tall panes, a bare prompt row on short ones.
pub(crate) fn composer_cells(
    input: &str,
    agents: &[AgentInfo],
    cols: u16,
    rows: u16,
) -> Vec<CellView> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    if composer_rows(rows) == 1 || cols < 6 {
        return prompt_cells(input, agents, 0, cols, rows - 1);
    }
    let t = crew_theme::theme();
    let top = rows - 3;
    let mut cells: Vec<CellView> = crate::boxdraw::titled_card(
        cols,
        3,
        "",
        t.border_normal,
        crate::palette::accent(),
        t.page_bg,
    )
    .into_iter()
    .map(|mut c| {
        c.row += top;
        c
    })
    .collect();
    chips_on_border(&mut cells, agents, cols, top);
    // Interior prompt, kept clear of the right border at `cols - 1`.
    cells.extend(prompt_cells(input, agents, 2, cols - 1, rows - 2));
    hints_on_border(&mut cells, cols, rows - 1);
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
