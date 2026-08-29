//! What rides the input bar's two border rows: the working directory (and the
//! focus-mode tag) on the top rule, and the focused pane's name — or a
//! transient status flash — on the bottom one.
//!
//! Split out of `inputbar_render` because both slots have the same job and
//! neither is text: they are *tags on a frame*. The bottom one used to be
//! clipped to `cols - 4`, which is the whole bar, so a pane whose title is a
//! full command line (`cargo test --workspace -p crew-app --bin crew`) turned
//! the bottom border into a second line of prose. A tag gets a tag's budget.
use crate::chatwidth::clip_w;

/// Columns a bottom-border tag may occupy. A third of the bar, never more
/// than [`TAG_MAX`] and never so small that a name is only its ellipsis — the
/// rule has to still read as a rule on either side of it.
const TAG_MAX: usize = 28;
const TAG_MIN: usize = 8;

pub(crate) fn tag_budget(cols: u16) -> usize {
    ((cols as usize) / 3).clamp(TAG_MIN, TAG_MAX)
}

/// The top-border legend: the working directory, with a standing `focus` tag
/// in front of it while focus mode is on (a mode owes the user a sign it is
/// on, and this legend is the only chrome always on screen).
/// `reserved` is what something else has already claimed on this rule — the
/// history tag at its right end. Without it a deep path fills the budget all
/// the way to the corner and the tag, drawn after, silently eats its tail.
pub(crate) fn top(cwd: &std::path::Path, cols: u16, reserved: usize) -> String {
    let budget = crate::boxdraw::title_budget(cols).saturating_sub(reserved);
    let path = if cwd.as_os_str().is_empty() {
        String::new()
    } else {
        // Keep the tail (current dir) when the path is deeper than the card.
        crate::cwd::fit_legend(&crate::cwd::display(cwd), budget)
    };
    if !crate::focusmode::on() {
        return path;
    }
    let tag = "\u{25c9} focus";
    if path.is_empty() {
        return tag.to_string();
    }
    let room = budget.saturating_sub(tag.chars().count() + 3);
    format!("{tag} \u{b7} {}", crate::cwd::fit_legend(&path, room))
}

/// The bottom-border tag, right-aligned: a transient status while one is
/// flashing, else the focused pane's name. A status is a moment; the pane
/// legend is the bar's standing answer to "where does this go?", so the flash
/// borrows the slot and gives it back.
///
/// A *pending confirmation* outranks both: `/closeall` and `/only` arm a
/// ten-second window in which running them again does the irreversible
/// thing, and that is a state the bar owes you a standing sign of, not a
/// three-second flash. It wears the bell colour, because it is a warning.
///
/// The other two get different budgets, because they are different kinds of thing.
/// A status is a sentence the bar is saying to you once — it may have the
/// whole rule if it needs it. A pane name is a standing label you read at a
/// glance, so it takes [`tag_budget`] and the rule keeps the rest.
///
/// Returns the already-clipped label (spaces included, so the rule breaks
/// cleanly around it) and its colour.
pub(crate) fn bottom(
    pending: Option<&str>,
    status: Option<&str>,
    pane: Option<&str>,
    cols: u16,
) -> Option<(String, (u8, u8, u8))> {
    let (text, budget, fg) = match pending.or(status) {
        Some(s) if pending.is_some() => (
            s,
            (cols as usize).saturating_sub(4),
            crew_theme::theme().bell,
        ),
        Some(s) => (
            s,
            (cols as usize).saturating_sub(4),
            crew_theme::theme().status_fg,
        ),
        None => (
            pane.map(str::trim).filter(|n| !n.is_empty())?,
            tag_budget(cols),
            crew_theme::theme().legend_off,
        ),
    };
    Some((format!(" {} ", clip_w(text, budget)), fg))
}

/// Where you are while browsing the bar's history with Up/Down — the one
/// border slot in crew that was always empty, at the right end of the top
/// rule.
///
/// Up does not simply walk back through everything typed: it recalls only
/// lines starting with whatever was in the bar when browsing began (the
/// zsh/fish rule). Both halves of that were invisible. The recalled line
/// looks exactly like a line you typed, so nothing said you were in history
/// at all; nothing said a prefix was filtering it; and at the oldest match Up
/// stops doing anything, which is indistinguishable from a broken key.
///
/// `hist 2/5 · git` says all three. `None` when not browsing.
pub(crate) fn history_tag(
    history: &[String],
    prefix: &str,
    pos: Option<usize>,
    cols: u16,
) -> Option<(String, (u8, u8, u8))> {
    let pos = pos?;
    // Matches are counted oldest-first, so `n of total` counts *back* from
    // the most recent — the direction Up travels.
    let matches: Vec<usize> = history
        .iter()
        .enumerate()
        .filter(|(_, h)| h.starts_with(prefix))
        .map(|(i, _)| i)
        .collect();
    let total = matches.len();
    let back = matches.iter().rev().position(|&i| i == pos)? + 1;
    let label = match prefix.is_empty() {
        true => format!("hist {back}/{total}"),
        false => format!(
            "hist {back}/{total} \u{b7} {}",
            clip_w(prefix, tag_budget(cols) / 2)
        ),
    };
    Some((
        format!(" {} ", clip_w(&label, tag_budget(cols))),
        crew_theme::theme().status_fg,
    ))
}

#[cfg(test)]
#[path = "inputlegend_tests.rs"]
mod tests;
