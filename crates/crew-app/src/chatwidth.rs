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

/// [`clip_w`] for prose: the cut drops a partial ` · ` segment whole, else a
/// partial word, when that boundary lies in the back half — `4 open items ·
/// 1 overd…` said a word that does not exist and half a count; `4 open
/// items…` says less, and all of it true. A single long token still cuts at a
/// letter.
pub(crate) fn clip_words(s: &str, max: usize) -> String {
    let cut = clip_w(s, max);
    let Some(body) = cut.strip_suffix('\u{2026}').filter(|_| str_w(s) > max) else {
        return cut;
    };
    // Cut already on a word's end (`macOS · up 2d…` of `… 2d 4h`): keep it.
    if s[body.len()..].starts_with(' ') {
        return format!(
            "{}\u{2026}",
            body.trim_end_matches([' ', '\u{b7}', ',', ';', ':'])
        );
    }
    let half = body.len() / 2;
    let at = |sep: &str| body.rfind(sep).filter(|&i| i >= half);
    match at(" \u{b7} ").or_else(|| at(" ")) {
        Some(i) => format!(
            "{}\u{2026}",
            body[..i].trim_end_matches([' ', ',', ';', ':'])
        ),
        None => cut,
    }
}

/// Pull a hard wrap at `end` back to a word boundary: just after a space, or
/// after a `,` `;` `(`, when one lies in the back half of `start..end` — so
/// wrapped code breaks where an editor would, not mid-token (`a0: f3` /
/// `2, a1`, an arrow split into `-` and `>`). The row keeps its break
/// character, so the rows still partition the line exactly — the paint, the
/// copy and the offsets read it verbatim. Unchanged when `end` is the line's
/// end or no boundary is near enough.
pub(crate) fn soft_end(full: &[char], start: usize, end: usize) -> usize {
    if end >= full.len() || end <= start + 1 {
        return end;
    }
    let floor = start + (end - start).div_ceil(2);
    let after = |ok: fn(char) -> bool| (floor + 1..=end).rev().find(|&k| ok(full[k - 1]));
    // A space first — the next row then opens on a word, not on the space
    // after a comma — then the punctuation code breaks after.
    after(char::is_whitespace)
        .or_else(|| after(|c| matches!(c, ',' | ';' | '(')))
        .unwrap_or(end)
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

#[cfg(test)]
#[path = "softwrap_tests.rs"]
mod softwrap_tests;
