//! Display-width helpers for the chat views. The cell grid is width-aware
//! (a wide emoji/CJK glyph occupies two cells — its advance snaps to
//! 2 × cell at render), so wrapping and column placement must count display
//! columns, not chars, or text after a wide glyph overlaps it.
use unicode_width::UnicodeWidthChar;

/// Display columns `c` occupies in the cell grid (0 for zero-width marks).
pub(crate) fn char_w(c: char) -> usize {
    UnicodeWidthChar::width(c).unwrap_or(0)
}

/// The furthest `end` such that `full[start..end]` fits `cols` display
/// columns. Always advances at least one char when any remain, so wrapping
/// loops can never stall on an over-wide glyph.
pub(crate) fn fit_end(full: &[char], start: usize, cols: usize) -> usize {
    let mut w = 0;
    let mut end = start;
    while end < full.len() {
        let cw = char_w(full[end]);
        if w + cw > cols {
            break;
        }
        w += cw;
        end += 1;
    }
    if end == start && start < full.len() {
        start + 1
    } else {
        end
    }
}

/// Total display columns of `s`.
pub(crate) fn str_w(s: &str) -> usize {
    s.chars().map(char_w).sum()
}

/// Truncate `s` to `max` display columns, keeping the head and marking the
/// cut with `…` — wide glyphs count two, so CJK clips on a cell boundary
/// rather than half a cell past it. The one clip used by every card legend
/// and toast body, so truncation reads the same everywhere on the canvas.
pub(crate) fn clip_w(s: &str, max: usize) -> String {
    if str_w(s) <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut w = 0;
    for c in s.chars() {
        if w + char_w(c) > max - 1 {
            break;
        }
        w += char_w(c);
        out.push(c);
    }
    out.push('\u{2026}');
    out
}

/// Place styled chars on one row from `start`, advancing by display width and
/// stopping before `max_col`; zero-width marks are skipped. Calls
/// `put(col, ch, style)` per placed char and returns the next free column.
pub(crate) fn place_row<S: Copy>(
    start: u16,
    max_col: u16,
    chars: impl IntoIterator<Item = (char, S)>,
    mut put: impl FnMut(u16, char, S),
) -> u16 {
    let mut x = start;
    for (ch, style) in chars {
        let w = char_w(ch) as u16;
        if w == 0 {
            continue;
        }
        if x + w > max_col {
            break;
        }
        put(x, ch, style);
        x += w;
    }
    x
}

#[cfg(test)]
#[path = "chatwidth_tests.rs"]
mod tests;
