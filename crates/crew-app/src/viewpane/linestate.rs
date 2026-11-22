//! The lines for a pane's state: a banner while it loads or fails, the rung's lines once it is
//! ready. Split from `lines.rs`, which is the rungs, for the line cap.
use crate::chatbody::CardLine;
use crate::viewpane::outline::Mark;
use crate::viewpane::LoadState;

/// Lines for the pane's current state at `cols` columns. `raw` shows text
/// unrendered (the `s` toggle); it changes the `Markdown` and `Csv` rungs —
/// the two whose default rendering shows something OTHER than the bytes
/// themselves — and leaves every other rung alone, since those already show
/// the bytes as they are.
#[cfg(test)]
pub(crate) fn for_state(
    state: &LoadState,
    raw: bool,
    cols: usize,
    invisibles: bool,
    split: bool,
) -> (Vec<CardLine>, Vec<Mark>, Vec<crate::chatmd::Picture>) {
    for_state_revealed(state, raw, cols, invisibles, split, None)
}

/// [`for_state`], with the markdown block `reveal` (in the FILE's bytes) drawn as its source —
/// the block an editing caret is in ([`crate::md::reveal`]).
pub(crate) fn for_state_revealed(
    state: &LoadState,
    raw: bool,
    cols: usize,
    invisibles: bool,
    split: bool,
    reveal: Option<crate::md::reveal::Reveal>,
) -> (Vec<CardLine>, Vec<Mark>, Vec<crate::chatmd::Picture>) {
    let t = crew_theme::theme();
    match state {
        LoadState::Loading { .. } => (
            vec![super::lines::banner("loading…", cols)],
            Vec::new(),
            Vec::new(),
        ),
        LoadState::Failed(msg) => (
            vec![super::linepaint::row(msg, t.ink, false)],
            Vec::new(),
            Vec::new(),
        ),
        LoadState::Ready { format, loaded } => {
            super::lines::ready_lines(*format, loaded, raw, cols, (invisibles, split), reveal)
        }
    }
}
