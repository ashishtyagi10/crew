//! The git badge on a pane's top border: `main ●3 ↑2 ↓1`.
//!
//! Every pane already knows which directory it is in — the badge says what
//! *state* that directory is in, on the card you are already looking at,
//! instead of only for crew's own cwd in the sidebar. A pane running an agent
//! in one worktree and a pane running tests in another are the case this
//! exists for: the two look identical until one of them says `+7`.
//!
//! The border is a scarce row, so the badge degrades in a fixed order as the
//! card narrows — arrows first, then the dirty count, then the branch itself,
//! which truncates. Deciding that per width by eye is how the settings form
//! ended up with fields clipped at their own default width.
use crew_render::CellView;

use crate::git::GitInfo;
use crate::panecard::put;

/// A piece of the badge and the ink it wears.
pub(crate) type Seg = (String, (u8, u8, u8));

/// The branch, ahead/behind arrows and dirty marker as separate strings, in
/// draw order — widest form first, before any budget is applied.
fn full(info: &GitInfo) -> Vec<Seg> {
    let t = crew_theme::theme();
    let mut out = vec![(info.branch.clone(), t.text_muted)];
    if info.changed > 0 {
        out.push((format!("\u{25cf}{}", info.changed), t.status_fg));
    }
    if info.ahead > 0 {
        out.push((format!("\u{2191}{}", info.ahead), t.text_muted));
    }
    if info.behind > 0 {
        out.push((format!("\u{2193}{}", info.behind), t.text_muted));
    }
    out
}

/// Display width of `segs` drawn with one space between them.
fn width(segs: &[Seg]) -> usize {
    let chars: usize = segs.iter().map(|(s, _)| s.chars().count()).sum();
    chars + segs.len().saturating_sub(1)
}

/// The badge that fits in `budget` columns, dropping detail in a fixed order:
/// behind, ahead, dirty count, then the branch truncated with an ellipsis.
/// `None` when even a truncated branch would be unreadable.
pub(crate) fn fit(info: &GitInfo, budget: usize) -> Option<Vec<Seg>> {
    let mut segs = full(info);
    while width(&segs) > budget && segs.len() > 1 {
        segs.pop();
    }
    if width(&segs) <= budget {
        return Some(segs);
    }
    // Only the branch is left and it is still too wide. Four columns buys
    // three characters and an ellipsis; less than that says nothing at all —
    // `m…` is not a branch name, it is noise on a border.
    if budget < 4 {
        return None;
    }
    let (branch, fg) = segs.pop()?;
    let kept: String = branch.chars().take(budget - 1).collect();
    Some(vec![(format!("{kept}\u{2026}"), fg)])
}

/// Draw the badge right-aligned so it ENDS at column `rx`, provided it fits
/// without reaching `min_col` (where the legend ends). Returns the next free
/// column to its left, unchanged when nothing was drawn.
pub(crate) fn draw(v: &mut Vec<CellView>, rx: u16, min_col: u16, info: &GitInfo) -> u16 {
    let budget = usize::from(rx.saturating_sub(min_col));
    let Some(segs) = fit(info, budget) else {
        return rx;
    };
    let w = width(&segs) as u16;
    let mut col = rx + 1 - w;
    let start = col;
    for (i, (text, fg)) in segs.iter().enumerate() {
        if i > 0 {
            col += 1;
        }
        for ch in text.chars() {
            put(v, col, 0, ch, *fg, false);
            col += 1;
        }
    }
    start.saturating_sub(2)
}

#[cfg(test)]
#[path = "gitbadge_tests.rs"]
mod tests;
