//! The left-nav UPDATE card interior: a spinner + stage line and a version
//! transition line, shown only while a `/update` runs (and briefly after). The
//! bordered fieldset frame is drawn by `panecard::push_card`; this fills it.
use crew_render::CellView;

use crate::palette::accent;
use crate::update::{Stage, UpdateState, SPINNER};

const TEXT: (u8, u8, u8) = (185, 190, 200);
const BG: (u8, u8, u8) = (0, 0, 0);

/// Interior cells for the UPDATE card: line 0 = spinner/result, line 1 = detail.
pub(crate) fn update_cells(u: &UpdateState, cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 4 || rows == 0 {
        return Vec::new();
    }
    let current = env!("CARGO_PKG_VERSION");
    let spin = SPINNER[u.spinner % SPINNER.len()];
    let (lead, head, detail) = match &u.stage {
        Stage::Checking => (spin, "checking…".to_string(), format!("v{current}")),
        Stage::Downloading(v) => (
            spin,
            "downloading".to_string(),
            format!("v{current} → v{v}"),
        ),
        Stage::Done(v) => ('✓', format!("updated v{v}"), "restart to apply".to_string()),
        Stage::Note(msg) => ('·', msg.clone(), String::new()),
    };
    let max = cols.saturating_sub(1);
    let mut out = Vec::new();
    out.push(glyph(0, 0, lead, accent()));
    write(&mut out, &head, 2, 0, TEXT, max);
    if rows > 1 && !detail.is_empty() {
        write(&mut out, &detail, 2, 1, TEXT, max);
    }
    out
}

fn glyph(col: u16, row: u16, c: char, fg: (u8, u8, u8)) -> CellView {
    CellView {
        col,
        row,
        c,
        fg,
        bg: BG,
        bold: false,
        italic: false,
    }
}

/// Write `s` at `(col, row)`, stopping before `max_col`.
fn write(out: &mut Vec<CellView>, s: &str, col: u16, row: u16, fg: (u8, u8, u8), max_col: u16) {
    for (i, c) in s.chars().enumerate() {
        let x = col + i as u16;
        if x >= max_col {
            break;
        }
        out.push(glyph(x, row, c, fg));
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
    fn done_shows_restart_note() {
        let cells = stage_cells(Stage::Done("9.9.9".into()));
        assert!(cells.iter().any(|c| c.c == '✓'), "success glyph present");
        assert!(row_text(&cells, 1).contains("restart"));
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
