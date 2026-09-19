//! What each pane crew DRAWS is doing, in one line — the answer the
//! minimized strip asks every pane for.
//!
//! v0.22.61 gave a thumbnail the pane's last line, which a terminal has
//! (its own bottom row) and a chat has (the agent it waits on). The six panes
//! crew draws itself had nothing to give and said nothing, so half the strip
//! stayed as empty as it had always been — and those are exactly the panes
//! whose whole state is a handful of numbers already on their own header row.
//!
//! One line each, from what the pane already holds: no new state, no work per
//! frame beyond the arithmetic the pane's own header does.
use crate::pane::PaneContent;

/// The line for a drawn pane, or `None` when it genuinely has nothing to say
/// (a viewer still loading, a swarm with no goal yet).
pub(crate) fn of(content: &PaneContent) -> Option<String> {
    match content {
        PaneContent::Todo(t) => todo(t),
        PaneContent::View(v) => view(v),
        PaneContent::Far(f) => far(f),
        PaneContent::Disk(d) => disk(d),
        PaneContent::Usage(u) => Some(u.glance()),
        PaneContent::Dash(d) => Some(d.glance()),
        PaneContent::Swarm(s) => swarm(s.state()),
        // A settings form is its own answer: every row of it is the state,
        // and "editing settings" is what the legend already says.
        _ => None,
    }
}

/// The list's own roll-up, minus the part a strip has no room for: what is
/// open and what is late is the reason you would go back to it.
fn todo(t: &crate::todopane::TodoPane) -> Option<String> {
    let now = crate::chattime::unix_now_ms();
    let (mut open, mut overdue) = (0u32, 0u32);
    for it in t.items.iter().filter(|i| !i.done) {
        open += 1;
        overdue += u32::from(it.due_ms.is_some_and(|d| d <= now));
    }
    match (open, overdue) {
        (0, _) => Some("nothing open".into()),
        (n, 0) => Some(crate::wording::count(n as usize, "open item")),
        (n, late) => Some(format!(
            "{} \u{b7} {late} overdue",
            crate::wording::count(n as usize, "open item")
        )),
    }
}

/// Where you are in the file, since the file's NAME is already the legend.
fn view(v: &crate::viewpane::ViewPane) -> Option<String> {
    let crate::viewpane::LoadState::Ready { loaded, .. } = &v.state else {
        return None;
    };
    let total = loaded.text.lines().count().max(1);
    let at = (v.scroll + 1).min(total);
    Some(format!("line {at} of {total}"))
}

/// The folder the active panel is in and how much is in it — a file manager
/// minimized mid-copy is one you are coming back to.
fn far(f: &crate::farpane::FarPane) -> Option<String> {
    let entries = match f.active {
        crate::farpane::Side::Left => f.left.entries.len(),
        crate::farpane::Side::Right => f.right.entries.len(),
    };
    Some(format!(
        "{} \u{b7} {}",
        f.active_panel_folder(),
        crate::wording::count(entries, "entry")
    ))
}

/// What the swarm is at: the goal while it plans, the tally while it runs,
/// the reason when it could not.
fn swarm(state: &crate::swarmpane::SwarmState) -> Option<String> {
    use crate::swarmpane::SwarmState;
    let t = match state {
        SwarmState::Planning { goal, .. } => return Some(format!("planning \u{b7} {goal}")),
        SwarmState::Failed { msg } => return Some(format!("failed \u{b7} {msg}")),
        SwarmState::Running { fleet, .. } => fleet.totals(),
    };
    let done = t.done + t.failed;
    let lost = match t.failed {
        0 => String::new(),
        n => format!(" \u{b7} {n} failed"),
    };
    Some(format!("{done} of {} done{lost}", done + t.live))
}

/// What the walk has found so far — the map's own header line, short.
fn disk(d: &crate::diskpane::DiskPane) -> Option<String> {
    let root = d.root();
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.to_string_lossy().into_owned());
    Some(format!("{name} \u{b7} {}", crate::disktile::bytes(d.total)))
}

#[cfg(test)]
#[path = "paneglance_tests.rs"]
mod tests;
