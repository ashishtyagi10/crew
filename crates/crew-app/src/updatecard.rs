//! The left-nav UPDATE card interior: a spinner + stage line and a version
//! transition line, shown only while a `/update` runs (and briefly after). The
//! bordered fieldset frame is drawn by `panelcard::push_card`; this fills it.
use crew_render::CellView;

use crate::palette::accent;
use crate::update::{Stage, UpdateState, SPINNER};

/// Interior cells for the UPDATE card: line 0 = spinner/result, line 1 = detail.
pub(crate) fn update_cells(u: &UpdateState, cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 4 || rows == 0 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let current = env!("CARGO_PKG_VERSION");
    let spin = SPINNER[u.spinner % SPINNER.len()];
    let max = cols.saturating_sub(1);
    let mut out = Vec::new();
    // A note is the only stage whose text has no bound: `update failed: {e}`
    // carries whatever the transport said, and this card is a narrow column
    // in the nav. It was written on ONE row and clipped mid-word with the
    // second row left blank — the one place an update reports a failure, and
    // the failure was the part that got cut. Notes wrap over every row the
    // card has and the last one ellipsises; a failure wears the bell colour
    // and its own lead, so it cannot be mistaken for "already up to date".
    if let Stage::Note(msg) = &u.stage {
        let failed = msg.starts_with(FAILED);
        let fg = if failed { t.bell } else { t.ink };
        out.push(glyph(0, 0, if failed { '!' } else { '·' }, fg, t.page_bg));
        for (i, line) in note_lines(msg, max.saturating_sub(2), rows)
            .iter()
            .enumerate()
        {
            write(&mut out, line, 2, i as u16, fg, max, t.page_bg);
        }
        return out;
    }
    let (lead, head, detail) = match &u.stage {
        Stage::Checking => (spin, "checking…".to_string(), format!("v{current}")),
        Stage::Downloading(v) => (
            spin,
            "downloading".to_string(),
            format!("v{current} → v{v}"),
        ),
        Stage::Done(v) => ('✓', format!("updated v{v}"), "restarting…".to_string()),
        // Handled above.
        Stage::Note(_) => unreachable!(),
    };
    out.push(glyph(0, 0, lead, accent(), t.page_bg));
    write(&mut out, &head, 2, 0, t.ink, max, t.page_bg);
    if rows > 1 && !detail.is_empty() {
        write(&mut out, &detail, 2, 1, t.ink, max, t.page_bg);
    }
    out
}

/// The prefix `update.rs` puts on a failed run — the one note that is bad news.
const FAILED: &str = "update failed";

/// `msg` greedily word-wrapped to `w` columns over at most `rows` lines, the
/// last ellipsised when there is more text than room.
fn note_lines(msg: &str, w: u16, rows: u16) -> Vec<String> {
    let w = w.max(1) as usize;
    let rows = rows.max(1) as usize;
    let mut out: Vec<String> = Vec::new();
    let mut rest = msg.trim();
    while !rest.is_empty() && out.len() < rows {
        let chars: Vec<char> = rest.chars().collect();
        let fit = crate::chatwidth::fit_end(&chars, 0, w);
        if fit >= chars.len() {
            out.push(rest.to_string());
            return out;
        }
        // The last row we have: say there is more rather than stopping
        // mid-word as if the message ended there.
        if out.len() + 1 == rows {
            let cut = crate::chatwidth::fit_end(&chars, 0, w.saturating_sub(1));
            let head: String = chars[..cut].iter().collect();
            out.push(format!("{head}\u{2026}"));
            return out;
        }
        let brk = chars[..fit]
            .iter()
            .rposition(|c| c.is_whitespace())
            .filter(|&i| i > 0)
            .unwrap_or(fit);
        out.push(
            chars[..brk]
                .iter()
                .collect::<String>()
                .trim_end()
                .to_string(),
        );
        rest = rest[chars[..brk].iter().map(|c| c.len_utf8()).sum::<usize>()..].trim_start();
    }
    out
}

fn glyph(col: u16, row: u16, c: char, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> CellView {
    CellView {
        col,
        row,
        c,
        fg,
        bg,
        bold: false,
        italic: false,
        ..Default::default()
    }
}

/// Write `s` at `(col, row)`, stopping before `max_col`.
fn write(
    out: &mut Vec<CellView>,
    s: &str,
    col: u16,
    row: u16,
    fg: (u8, u8, u8),
    max_col: u16,
    bg: (u8, u8, u8),
) {
    for (i, c) in s.chars().enumerate() {
        let x = col + i as u16;
        if x >= max_col {
            break;
        }
        out.push(glyph(x, row, c, fg, bg));
    }
}

#[cfg(test)]
#[path = "updatecard_tests.rs"]
mod tests;
