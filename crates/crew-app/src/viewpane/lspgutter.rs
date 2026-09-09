//! The margin beside a code file that says where the diagnostics are: `●`
//! on an error line, `▲` on a warning line, nothing elsewhere. Laid down the
//! way the blame column is ([`super::blamegutter`]): prepended to lines
//! already wrapped narrower by [`WIDTH`], keyed on the source line number
//! each row's gutter names.
use crate::chatbody::{plain, CardLine, Color};
use crew_lsp::{Diagnostic, Severity};

/// The mark and one space.
pub(crate) const WIDTH: usize = 2;

pub(crate) type Mark = (char, Color);

/// The mark a severity earns — errors and warnings only. Information and
/// hints would put a mark on every unused import and drown the two that
/// matter. The inks are the theme's floored bell (red) and status (amber).
pub(crate) fn mark_for(sev: Severity) -> Option<Mark> {
    let t = crew_theme::theme();
    match sev {
        Severity::Error => Some(('\u{25cf}', t.bell)),
        Severity::Warning => Some(('\u{25b2}', t.status_fg)),
        _ => None,
    }
}

/// One slot per source line (0-based), an error winning over a warning on
/// the same line.
pub(crate) fn marks(diags: &[Diagnostic]) -> Vec<Option<Mark>> {
    let last = diags
        .iter()
        .map(|d| d.range.start.line as usize)
        .max()
        .map_or(0, |n| n + 1);
    let mut out = vec![None; last];
    for d in diags {
        let Some(mark) = mark_for(d.severity) else {
            continue;
        };
        let slot = &mut out[d.range.start.line as usize];
        if slot.is_none() || d.is_error() {
            *slot = Some(mark);
        }
    }
    out
}

/// Prepend the mark column. `at` is where each row's number gutter starts —
/// the blame column's width when one is showing, since that was prepended
/// first — so the source line is read from the right cells.
pub(crate) fn apply(lines: &mut [CardLine], marks: &[Option<Mark>], at: usize) {
    for line in lines.iter_mut() {
        let mark = super::blamegutter::source_line_at(line, at)
            .and_then(|n| marks.get(n - 1).copied().flatten());
        let mut head: CardLine = match mark {
            Some((c, fg)) => vec![plain(c, fg, true), plain(' ', fg, false)],
            None => vec![plain(' ', (0, 0, 0), false); WIDTH],
        };
        head.append(&mut std::mem::take(line));
        *line = head;
    }
}

#[cfg(test)]
#[path = "lspgutter_tests.rs"]
mod tests;
