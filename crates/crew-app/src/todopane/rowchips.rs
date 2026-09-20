//! What the VIEW tells a row about itself, and which chips the row draws
//! because of it.
//!
//! A row's right side is not a property of the item alone. The done history
//! puts a tick time where the due would go, and under an `#assignee` band
//! the header already names the owner — `#sam` on the header and again on
//! every row beneath it is the same word three times, and on a narrow tile
//! it is the chip that pushes the row into stacking, while `@project` (the
//! one thing the band does NOT tell you) is the one dropped for want of
//! room. So the view hands every width answer the same small context.
use super::item::TodoItem;
use super::{Bands, TodoPane};

/// What the view tells a row. Both bits come from the pane rather than the
/// item, and every width answer needs the pair — as two bare bools at a
/// call site they are two chances to pass them the wrong way round.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct RowCtx {
    /// The done-history log: the tick time rides where the due would.
    pub done_view: bool,
    /// A `#assignee` band header stands over this row and names its owner.
    pub banded: bool,
}

impl TodoPane {
    /// This pane's row context. Banding is a property of the LIST, not of
    /// the row: when the list bands, every row sits under a header, and the
    /// `unassigned` bucket's rows have no owner to drop anyway.
    pub(crate) fn rowctx(&self) -> RowCtx {
        RowCtx {
            done_view: self.done_view,
            banded: self.bands() == Bands::People,
        }
    }
}

/// The tag chips in PLACEMENT order — and the right side is laid right to
/// left, so this is outermost first: `@project`, then `#assignee` inside it.
/// The owner therefore lands nearest the title, which is the column the eye
/// runs down on a list you read by person.
fn tag_chips(it: &TodoItem, owner: bool) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(tag) = &it.project {
        out.push(format!("@{tag}"));
    }
    if let Some(w) = it.assignee.as_ref().filter(|_| owner) {
        out.push(format!("#{w}"));
    }
    out
}

/// EVERY tag the item carries — the round-trip list `e` rebuilds the
/// composer from. The owner has to survive whatever shape the list happened
/// to be in: editing an item under its own band must not file it back
/// belonging to nobody.
pub(crate) fn chips(it: &TodoItem) -> Vec<String> {
    tag_chips(it, true)
}

/// The chips a row DRAWS: the same tags, minus the owner a band overhead
/// has already named — plus `▶agent` when the item has been handed to an
/// agent. The run chip is a fact about what happened, not a tag the user
/// typed, so it is here and not in [`chips`]: `e` must not carry it back
/// into the composer.
pub(crate) fn row_chips(it: &TodoItem, ctx: RowCtx) -> Vec<String> {
    let mut out = tag_chips(it, !ctx.banded);
    if let Some(run) = &it.run {
        out.push(format!("{RUN}{}", run.agent));
    }
    out
}

/// The sigil the run chip wears while the agent runs: `▶claude`.
pub(crate) const RUN: char = '\u{25b6}';
/// …and once it has exited or its pane is gone: `▷claude`. The list is not
/// told the task is done — an agent's exit is not that — only that nobody
/// is working on it right now.
pub(crate) const RAN: char = '\u{25b7}';

/// The name behind a chip's sigil: `@crew` → `crew`, `▶claude` → `claude`.
/// By CHAR, not byte — `▶` is three bytes, and `&chip[1..]` on it panicked
/// the first time a run chip was drawn (v0.22.79).
pub(crate) fn chip_name(chip: &str) -> &str {
    chip.char_indices().nth(1).map_or("", |(i, _)| &chip[i..])
}

impl TodoPane {
    /// Whether the agent this item was handed to is still at work.
    pub(crate) fn running(&self, it: &TodoItem) -> bool {
        it.run.as_ref().is_some_and(|r| self.live.contains(&r.pane))
    }

    /// [`row_chips`] as the row DRAWS them: the run chip's sigil follows the
    /// agent's state. Same width either way, so the fit is measured once.
    pub(crate) fn live_chips(&self, it: &TodoItem) -> Vec<String> {
        let mut chips = row_chips(it, self.rowctx());
        if let Some(last) = chips.last_mut().filter(|c| c.starts_with(RUN)) {
            if !self.running(it) {
                *last = format!("{RAN}{}", chip_name(last));
            }
        }
        chips
    }
}

/// Whether the row carries anything on its right side at all.
pub(crate) fn has_chips(it: &TodoItem, ctx: RowCtx) -> bool {
    !row_chips(it, ctx).is_empty()
        || if ctx.done_view {
            it.done_ms.is_some()
        } else {
            it.due_ms.is_some()
        }
}

#[cfg(test)]
#[path = "rowchips_tests.rs"]
mod tests;
