//! The chat's word wrap ([`wrap_indices`]), split from `chatlayout` (line
//! cap) when it learned the prose rule about separators.

/// Word-aware wrap of `full` to width `cols`: the `[start, end)` char ranges of
/// each line. Words longer than `cols` are hard-broken; the single space at a
/// wrap point is dropped; no row opens on a spaced `·` or dash. Always
/// returns at least one (possibly empty) line.
pub(crate) fn wrap_indices(full: &[char], cols: usize) -> Vec<(usize, usize)> {
    if cols == 0 || full.is_empty() {
        return vec![(0, full.len())];
    }
    let n = full.len();
    let mut lines = Vec::new();
    let mut start = 0;
    while start < n {
        // Width-aware: a wide glyph counts two columns (see `chatwidth`).
        let max_end = crate::chatwidth::fit_end(full, start, cols);
        if max_end == n {
            lines.push((start, n));
            break;
        }
        // Line is full; prefer breaking at the last space within it — and
        // never so the next row opens on a list's `·` or a spaced dash: the
        // prose rule (`md::dashbreak`), which the chat's bodies, the
        // listings and the toasts wrap by too.
        let space = full[start..max_end].iter().rposition(|&c| c == ' ');
        let safe = crate::md::dashbreak::dash_safe(full, start, space, cols)
            .filter(|&q| start + q <= max_end);
        match safe.or(space) {
            Some(p) if p > 0 => {
                lines.push((start, start + p));
                start += p + 1; // skip the break space
            }
            _ => {
                lines.push((start, max_end)); // a too-long word: hard break
                start = max_end;
            }
        }
    }
    lines
}

#[cfg(test)]
#[path = "wrapidx_tests.rs"]
mod tests;
