//! The crew pane's input composer. Tall panes get a bordered fieldset card:
//! the addressable `@agents` ride the top border as the legend (each in its
//! roster colour) and the `❯` prompt sits on the interior row (a valid leading
//! `@agent` mention takes that agent's colour). Short panes fall back to a
//! single bare prompt row.
use crew_plugin::AgentInfo;
use crew_render::CellView;

/// Rows the composer occupies for this input at this pane size: the bordered
/// card (top border + one row per wrapped input line + bottom border) grows
/// with the input, capped at a third of the pane so the transcript keeps the
/// room. Short/narrow panes fall back to a single bare prompt row.
pub(crate) fn composer_rows(input: &str, cols: u16, rows: u16) -> u16 {
    if rows < 7 || cols < 6 {
        return 1;
    }
    let lines = wrap_ranges(input, text_width(2, cols - 1)).len();
    (lines.min((rows / 3) as usize).max(1) as u16) + 2
}

/// Text columns per interior prompt line: between the `❯ ` prompt (2 cols
/// after `x0`) and the clip edge `max`.
fn text_width(x0: u16, max: u16) -> usize {
    (max as usize).saturating_sub(x0 as usize + 2).max(1)
}

/// Char-index ranges of `input` split into prompt lines `width` columns wide:
/// hard-broken at `\n`, soft-wrapped by display width in between. Always at
/// least one (possibly empty) line so the prompt row exists while empty.
pub(crate) fn wrap_ranges(input: &str, width: usize) -> Vec<(usize, usize)> {
    let chars: Vec<char> = input.chars().collect();
    let mut lines = Vec::new();
    let mut seg = 0;
    for (i, &c) in chars.iter().enumerate() {
        if c == '\n' {
            wrap_segment(&mut lines, &chars, seg, i, width);
            seg = i + 1;
        }
    }
    wrap_segment(&mut lines, &chars, seg, chars.len(), width);
    lines
}

/// Append the soft-wrapped line ranges of `chars[start..end]` (one `\n`-free
/// segment; an empty segment still yields its one empty line).
fn wrap_segment(
    lines: &mut Vec<(usize, usize)>,
    chars: &[char],
    start: usize,
    end: usize,
    width: usize,
) {
    if start == end {
        lines.push((start, end));
        return;
    }
    let mut s = start;
    while s < end {
        let e = crate::chatwidth::fit_end(&chars[..end], s, width);
        lines.push((s, e));
        s = e;
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

/// The dim hint shown in place of the caret/typed text while the composer is
/// empty — purely visual: it's never written into the input buffer, so typing
/// simply replaces it and it never touches the cursor or what Enter submits.
const PLACEHOLDER_HINT: &str = "type a task \u{00b7} / for constructs \u{00b7} @ to pick an agent";

/// Placeholder cells for [`PLACEHOLDER_HINT`], starting at column `x0`,
/// clipped to `[x0, max)` (truncated on panes narrower than the hint).
fn placeholder_cells(x0: u16, max: u16, row: u16) -> Vec<CellView> {
    let muted = crew_theme::theme().text_muted;
    (x0..max)
        .zip(PLACEHOLDER_HINT.chars())
        .map(|(x, c)| cell(x, row, c, muted, false))
        .collect()
}

/// The `❯ input▏` prompt block: up to `nrows` wrapped input lines starting at
/// `first_row`, each clipped to `[x0+2, max)`, with a valid `@mention`
/// coloured. When the input wraps past `nrows` the view follows the caret —
/// the LAST lines show, since editing always happens at the end. The `❯`
/// marks the first visible row; the caret rides the last one.
fn prompt_lines(
    input: &str,
    agents: &[AgentInfo],
    x0: u16,
    max: u16,
    first_row: u16,
    nrows: u16,
) -> Vec<CellView> {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let mut cells = vec![cell(x0, first_row, '\u{276f}', accent, true)]; // ❯
    if input.is_empty() {
        cells.extend(placeholder_cells(x0 + 2, max, first_row));
        return cells;
    }
    let mention = mention_len(input, agents);
    let m_color = if mention > 0 {
        crate::chatroster::agent_color(&input[1..mention])
    } else {
        t.ink
    };
    // Mid-message @file mentions read as chips: tinted accent while typed.
    let spans = crate::chatmention::spans(input);
    let in_span = |i: usize| spans.iter().any(|&(s, e)| i >= s && i < e);
    let style_at = |i: usize| {
        if i < mention {
            (m_color, true)
        } else if in_span(i) {
            (accent, false)
        } else {
            (t.ink, false)
        }
    };
    let chars: Vec<char> = input.chars().collect();
    let ranges = wrap_ranges(input, text_width(x0, max));
    let skip = ranges.len().saturating_sub(nrows.max(1) as usize);
    let (mut end_x, mut end_row) = (x0 + 2, first_row);
    for (li, &(s, e)) in ranges[skip..].iter().enumerate() {
        let row = first_row + li as u16;
        let styled = chars[s..e]
            .iter()
            .enumerate()
            .map(|(j, &c)| (c, style_at(s + j)));
        // Width-aware placement: a wide glyph advances two columns, and the
        // caret lands after the text instead of on top of it.
        let x = crate::chatwidth::place_row(x0 + 2, max, styled, |x, c, (fg, bold)| {
            cells.push(cell(x, row, c, fg, bold))
        });
        (end_x, end_row) = (x, row);
    }
    if end_x < max {
        cells.push(cell(end_x, end_row, '\u{258f}', accent, false)); // ▏ caret
    }
    cells
}

/// Overlay the `@agent` chips on the card's top border as its legend
/// (`╭─ @planner @coder ─╮`), each chip in its roster colour. Returns the
/// first free column after the chips (or after the corner, if none fit) so
/// callers can place more content on the same row without overlapping them.
fn chips_on_border(cells: &mut Vec<CellView>, agents: &[AgentInfo], cols: u16, row: u16) -> u16 {
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
        x += 1;
    }
    x
}

/// A dim `Nc` badge shown at the right of the composer when input is long.
fn char_count_badge(len: usize) -> Option<String> {
    (len > 120).then(|| format!("{len}c"))
}

/// Right-align the char-count badge on the card's top border, clear of the
/// agent chips (`chips_end`) and the right corner; skipped entirely if the
/// row is too narrow to fit it without overlapping either.
fn badge_on_border(cells: &mut Vec<CellView>, badge: &str, chips_end: u16, cols: u16, row: u16) {
    let muted = crew_theme::theme().text_muted;
    let w = badge.chars().count() as u16;
    if cols < 3 + w {
        return;
    }
    let start = cols - 2 - w;
    if start <= chips_end {
        return;
    }
    for (x, c) in (start..).zip(badge.chars()) {
        cells.push(cell(x, row, c, muted, false));
    }
}

/// Render the composer into the bottom `composer_rows(input, cols, rows)`
/// rows: a bordered fieldset card that grows with the wrapped input on tall
/// panes, a bare prompt row on short ones.
pub(crate) fn composer_cells(
    input: &str,
    agents: &[AgentInfo],
    cols: u16,
    rows: u16,
) -> Vec<CellView> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    let total = composer_rows(input, cols, rows);
    if total == 1 {
        return prompt_lines(input, agents, 0, cols, rows - 1, 1);
    }
    let t = crew_theme::theme();
    let top = rows - total;
    let mut cells: Vec<CellView> = crate::boxdraw::titled_card(
        cols,
        total,
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
    let chips_end = chips_on_border(&mut cells, agents, cols, top);
    if let Some(badge) = char_count_badge(input.chars().count()) {
        badge_on_border(&mut cells, &badge, chips_end, cols, top);
    }
    // Interior prompt lines, kept clear of the right border at `cols - 1`.
    cells.extend(prompt_lines(input, agents, 2, cols - 1, top + 1, total - 2));
    cells
}

#[cfg(test)]
#[path = "chatinput_tests.rs"]
mod tests;

/// Pure input reducer.
///
/// - `enter`: return `Some(old_input)`, clear `input`.
/// - `backspace`: pop last char, return `None`.
/// - `ch=Some(c)` (non-control, or `\n` — Shift+Enter's newline): push `c`,
///   return `None`.
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
        if !c.is_control() || c == '\n' {
            input.push(c);
        }
        None
    } else {
        None
    }
}
