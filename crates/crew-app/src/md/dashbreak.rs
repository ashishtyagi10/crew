//! Where a prose row may break: never so the next one opens on a dash.

/// The break at `start + p`, moved back a word when the row after it would
/// open on a spaced dash — `— or draft a plan` belongs to the word before
/// it, and a row may end on `result —` but not start on the dash.
pub(super) fn dash_safe(full: &[char], start: usize, p: Option<usize>) -> Option<usize> {
    let p = p?;
    let opens_on_dash = full.get(start + p + 1) == Some(&'\u{2014}')
        && full.get(start + p + 2).is_none_or(|&c| c == ' ');
    if !opens_on_dash {
        return Some(p);
    }
    match full[start..start + p].iter().rposition(|&c| c == ' ') {
        Some(q) if q > 0 => Some(q),
        _ => Some(p),
    }
}
