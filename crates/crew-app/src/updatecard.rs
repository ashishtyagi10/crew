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
mod tests {
    use super::*;

    #[test]
    fn downloading_shows_version_transition() {
        // A Downloading stage renders the spinner lead and a "vCUR → vNEW" detail.
        let cells = stage_cells(Stage::Downloading("9.9.9".into()));
        let line1: String = row_text(&cells, 1);
        assert!(
            line1.contains("9.9.9"),
            "detail names target, got {line1:?}"
        );
        assert!(
            line1.contains('→'),
            "detail shows transition, got {line1:?}"
        );
    }

    #[test]
    fn done_says_a_restart_is_coming() {
        let cells = stage_cells(Stage::Done("9.9.9".into()));
        assert!(cells.iter().any(|c| c.c == '✓'), "success glyph present");
        assert!(row_text(&cells, 1).contains("restarting"));
    }

    /// The one place an update reports a failure used to clip the failure:
    /// the message went on ONE row of a narrow nav column and the second row
    /// stayed blank.
    #[test]
    fn a_long_note_uses_every_row_the_card_has() {
        let msg = "update failed: could not reach github.com (connection refused)";
        let cells = stage_cells(Stage::Note(msg.into()));
        let (r0, r1) = (row_text(&cells, 0), row_text(&cells, 1));
        assert!(r1.trim().len() > 4, "the second row is used: {r1:?}");
        assert!(
            r0.contains("failed") && r1.contains("github"),
            "the message continues onto it: {r0:?} / {r1:?}"
        );
        assert!(r1.ends_with('\u{2026}'), "and says there is more: {r1:?}");
        for row in [r0, r1] {
            assert!(row.chars().count() <= 24, "row overruns the card: {row:?}");
        }
    }

    /// A failure must not read like "already up to date".
    #[test]
    fn a_failed_note_wears_the_bell_colour_and_its_own_lead() {
        let _g = crate::app::theme_test_guard();
        let bad = stage_cells(Stage::Note("update failed: no such host".into()));
        let good = stage_cells(Stage::Note("already up to date (v1.2.3)".into()));
        let bell = crew_theme::theme().bell;
        assert!(bad.iter().any(|c| c.c == '!' && c.fg == bell));
        assert!(bad.iter().all(|c| c.fg == bell));
        assert!(good.iter().all(|c| c.fg != bell), "a note is not an alarm");
    }

    #[test]
    fn narrow_card_renders_nothing() {
        let u = UpdateState::for_test(Stage::Checking);
        assert!(update_cells(&u, 3, 2).is_empty());
    }

    fn stage_cells(stage: Stage) -> Vec<CellView> {
        update_cells(&UpdateState::for_test(stage), 24, 2)
    }

    fn row_text(cells: &[CellView], row: u16) -> String {
        let mut r: Vec<_> = cells.iter().filter(|c| c.row == row).collect();
        r.sort_by_key(|c| c.col);
        r.iter().map(|c| c.c).collect()
    }
}
