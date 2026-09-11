//! What the composer's legend says, and in what colour.
//!
//! Split out of [`super::composer`] for the line cap. The legend is the
//! composer's one line of feedback — a recognised due date, an edit in
//! progress, the filters the list is under — and choosing between them is a
//! different job from drawing the box it sits on.
use super::{duedate, TodoPane};

/// The legend and its colour for the current composer state, in precedence
/// order: the history view names itself, an edit in progress says so, a
/// recognised due date previews its parse exactly as the row will print it,
/// then the active filters, then plain `new`.
pub(crate) fn text(p: &TodoPane, hit: Option<&duedate::DueHit>) -> (String, (u8, u8, u8)) {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    // Both filters, each under its own sigil — `@crew #priya` is one
    // person's work on one project and the legend has to be able to say so.
    let tags: Vec<String> = [
        p.filter.as_ref().map(|f| format!("@{f}")),
        p.who.as_ref().map(|w| format!("#{w}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    // One tag colours the legend as itself; two would have to pick, so the
    // pair falls back to the resting tone.
    let tag_fg = match tags.as_slice() {
        [one] => crew_theme::tag_color(&one[1..], t),
        _ => t.legend_off,
    };
    if p.done_view {
        let head = std::iter::once("done".to_string()).chain(tags.iter().cloned());
        return (head.collect::<Vec<_>>().join(" "), tag_fg);
    }
    if p.editing.is_some() {
        return ("edit".to_string(), t.legend_off);
    }
    if let Some(h) = hit {
        // The parsed date alone, with no "due" in front of it: the legend
        // shows the same label the row will wear, and a word the row does
        // not carry only makes the two read as different things.
        let now = duedate::now_local();
        return (duedate::label_naive(h.due, h.has_time, now), accent);
    }
    if tags.is_empty() {
        return ("new".to_string(), t.legend_off);
    }
    (tags.join(" "), tag_fg)
}
