//! The tiers that fit the swarm status line's words to the pane: the
//! `" {title}… ({inner})"` shapes `chatswarmview::layout` tries in its
//! sacrifice order, and the strict display-width clamp they share. Split
//! from `chatswarmview` along that line for the 200-line cap.

/// Truncate `s` to at most `max_w` display columns using the same strict rule
/// as [`crate::chatwidth::place_row`] — a glyph that would straddle `max_w` is
/// dropped, never forced through — and return the kept prefix with its exact
/// display width. Pre-clamping here (rather than leaning on `place_row`'s
/// `max_col` at draw time) is what lets `chatswarmview::layout` know the left text's true
/// width for the bar and the right-aligned tokens.
pub(crate) fn clamp(s: &str, max_w: u16) -> (String, u16) {
    let mut w = 0u16;
    let mut out = String::new();
    for c in s.chars() {
        let cw = crate::chatwidth::char_w(c) as u16;
        if cw == 0 {
            out.push(c); // zero-width marks ride along, no column cost
            continue;
        }
        if w + cw > max_w {
            break;
        }
        w += cw;
        out.push(c);
    }
    (out, w)
}

/// `" {title}… ({inner})"` only when the *whole* title fits `budget` — the
/// tier that keeps a task's name intact. `None` if it would need truncating,
/// leaving the caller to try a cheaper `inner` first (drop elapsed) before
/// resorting to [`paren_with_title`], which does truncate.
pub(crate) fn paren_whole(title: &str, inner: &str, budget: u16) -> Option<String> {
    let s = format!(" {title}\u{2026} ({inner})");
    (crate::chatwidth::str_w(&s) as u16 <= budget).then_some(s)
}

/// `" {title}… ({inner})"` with the title truncated to fit `budget`, or `None`
/// when even a one-column title can't share the row with `({inner})`.
pub(crate) fn paren_with_title(title: &str, inner: &str, budget: u16) -> Option<String> {
    let inner_w = crate::chatwidth::str_w(inner) as u16;
    // Fixed punctuation around the title: leading space + "… (" + ")" = 5 cols.
    let fixed = 1 + 3 + 1;
    let title_budget = budget.checked_sub(inner_w + fixed)?;
    if title_budget == 0 {
        return None;
    }
    let (title, _) = clamp(title, title_budget);
    if title.is_empty() {
        return None;
    }
    Some(format!(" {title}\u{2026} ({inner})"))
}

/// `" ({inner})"` with no title, for panes too narrow to show one. `None` when
/// even that doesn't fit `budget`.
pub(crate) fn paren_bare(inner: &str, budget: u16) -> Option<String> {
    let inner_w = crate::chatwidth::str_w(inner) as u16;
    // Leading space + "(" + ")" = 3 cols.
    (inner_w + 3 <= budget).then(|| format!(" ({inner})"))
}
