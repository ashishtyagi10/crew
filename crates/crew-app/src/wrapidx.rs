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
        // A word that ends exactly at the edge breaks at the space after it:
        // looking only inside the row, a word as wide as the row was cut in
        // two and a full row gave its last word to the next one.
        let space = match full.get(max_end) {
            Some(' ') => Some(max_end - start),
            _ => full[start..max_end].iter().rposition(|&c| c == ' '),
        };
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

/// [`wrap_indices`] without a one-word last row when the row above can spare
/// a word (CSS's `text-wrap: pretty`). The keys panel's descriptions ended on
/// `bar`, `canvas` and `auto)` alone on a row at narrow widths: a paragraph
/// that looks finished a row early, and a row read as a binding of its own.
/// Only the last break moves, so every other row is the plain wrap's.
pub(crate) fn wrap_pretty(full: &[char], cols: usize) -> Vec<(usize, usize)> {
    let mut lines = wrap_indices(full, cols);
    let [.., (a, b), (c, d)] = lines[..] else {
        return lines;
    };
    // A hard-broken word's tail has no space before it to move a word over.
    if c != b + 1 || full[c..d].contains(&' ') {
        return lines;
    }
    let Some(p) = full[a..b]
        .iter()
        .rposition(|&ch| ch == ' ')
        .filter(|&p| p > 0)
    else {
        return lines;
    };
    let moved = a + p + 1;
    // Never onto a row that opens on a list's `·` or a dash (the prose rule).
    let opens_on_stop = matches!(full[moved], '\u{b7}' | '\u{2014}' | '\u{2013}');
    if opens_on_stop || crate::chatwidth::fit_end(full, moved, cols) < d {
        return lines;
    }
    let k = lines.len();
    lines[k - 2] = (a, a + p);
    lines[k - 1] = (moved, d);
    lines
}

#[cfg(test)]
#[path = "wrapidx_tests.rs"]
mod tests;
