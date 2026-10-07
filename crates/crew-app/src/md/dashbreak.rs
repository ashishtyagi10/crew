//! Where a prose row may break: never so the next one opens on a dash or on
//! a list's separator dot.

/// The break at `start + p` in a row `cols` wide, moved when the row after
/// it would open on a spaced dash or middle dot. `— or draft a plan`
/// belongs to the word before it, so the break moves back a word: a row may
/// end on `result —` but not start on the dash. A dot separates a list
/// (`3 done · 1 failed · 12s`), so the row ends ON it when it fits —
/// `1 failed ·` over `12s`, the way a comma sits — and otherwise on the last
/// dot that does, so no part of the list is split; with none, it moves back
/// a word like the dash.
pub(crate) fn dash_safe(
    full: &[char],
    start: usize,
    p: Option<usize>,
    cols: usize,
) -> Option<usize> {
    let p = p?;
    let opens = |mark: char| {
        full.get(start + p + 1) == Some(&mark) && full.get(start + p + 2).is_none_or(|&c| c == ' ')
    };
    let dot = opens('\u{b7}');
    if dot && p + 2 <= cols && full.get(start + p + 2).is_some() {
        return Some(p + 2);
    }
    let row = &full[start..start + p];
    let sep = |q: usize| row[q] == ' ' && row[q - 1] == '\u{b7}' && row[q - 2] == ' ';
    if let Some(q) = (2..p).rev().find(|&q| dot && sep(q)) {
        return Some(q);
    }
    if !dot && !opens('\u{2014}') {
        return Some(p);
    }
    match full[start..start + p].iter().rposition(|&c| c == ' ') {
        Some(q) if q > 0 => Some(q),
        _ => Some(p),
    }
}
