//! Row builders for the leading-token palettes (`/` and `@`) — split out of
//! `chatpalette.rs` (child module) to keep that file under the house line
//! cap, same pattern as `modelpick.rs`'s `modelbadge`/`modelrecents`.
use crate::chatcomplete::{describe, CONSTRUCTS};
use crate::suggest::MenuItem;

/// The palette's sections, in the order a session tends to need them: do the
/// work, look at what changed, manage the session, configure the stack.
///
/// Ungrouped, 24 constructs is a wall of equally-weighted rows. Grouped, the
/// eye finds the verb it wants by what it is FOR — which is what opencode's
/// palette does and why its longer list reads shorter. Anything not named
/// here still appears, under "other": a construct must never be invisible
/// because someone forgot to file it.
const SECTIONS: &[(&str, &[&str])] = &[
    // `/fan` and `/loop` retired: plain language reaches both through the
    // broker's intent router, so the palette no longer teaches them.
    ("work", &["/goal", "/plan"]),
    ("changes", &["/diff", "/commit", "/review", "/restore"]),
    (
        "session",
        &["/resume", "/memory", "/compact", "/export", "/stop"],
    ),
    (
        "setup",
        &["/model", "/skill", "/mcp", "/reload", "/doctor", "/theme"],
    ),
    ("help", &["/help", "/exit"]),
];

fn row(c: &str) -> MenuItem {
    MenuItem {
        label: c.to_string(),
        desc: describe(c).to_string(),
        fill: c.to_string(),
        submit: false,
        header: false,
        dim: false,
        needs: None,
    }
}

fn header(label: &str) -> MenuItem {
    MenuItem {
        label: label.to_string(),
        desc: String::new(),
        fill: String::new(),
        submit: false,
        header: true,
        dim: false,
        needs: None,
    }
}

pub(super) fn slash_items(query: &str) -> Vec<MenuItem> {
    let hits: Vec<&str> = CONSTRUCTS
        .iter()
        .copied()
        .filter(|c| c[1..].starts_with(query))
        .collect();
    // Headers only while BROWSING. Once the user is filtering they have
    // already said what they want, and section labels between one or two
    // survivors are chrome in the way of the answer.
    if !query.is_empty() {
        return hits.into_iter().map(row).collect();
    }
    let mut out = Vec::new();
    let mut placed: Vec<&str> = Vec::new();
    for (label, members) in SECTIONS {
        let present: Vec<&str> = members
            .iter()
            .copied()
            .filter(|m| hits.contains(m))
            .collect();
        if present.is_empty() {
            continue;
        }
        out.push(header(label));
        for c in present {
            placed.push(c);
            out.push(row(c));
        }
    }
    let rest: Vec<&str> = hits.into_iter().filter(|c| !placed.contains(c)).collect();
    if !rest.is_empty() {
        out.push(header("other"));
        out.extend(rest.into_iter().map(row));
    }
    out
}

/// Rows for the leading `@`: the full attach picker (agents, skills, files
/// — `chatmention::filter`'s section order), agents only once the token has
/// a `+` (multi-target selectors route, they don't attach).
pub(super) fn attach_items(
    query: &str,
    entries: &[crate::chatmention::MentionEntry],
    multi: bool,
) -> Vec<MenuItem> {
    use crate::chatmention::MentionEntry;
    let hits = crate::chatmention::filter(entries, query);
    let agents = hits
        .iter()
        .filter(|e| matches!(e, MentionEntry::Agent { .. }))
        .count();
    let mut out: Vec<MenuItem> = hits
        .into_iter()
        .filter(|e| !multi || matches!(e, MentionEntry::Agent { .. }))
        .map(|e| MenuItem {
            label: format!("@{}", e.token()),
            desc: e.desc(),
            fill: e.token(),
            submit: false,
            header: false,
            dim: false,
            needs: None,
        })
        .collect();
    // `@a+b` fans one task out to several agents at once. It was implemented,
    // filtered for correctly two lines up, and documented in exactly one
    // place the app never shows — so the popup says it, at the moment the
    // user is looking at the agents it applies to.
    //
    // Only when there are two to chain, and not while already chaining: the
    // hint is for someone who does not know the idiom, and repeating it to
    // someone mid-selector is noise.
    if !multi && agents >= 2 && !out.is_empty() {
        out.push(header("@a+b  fans the task out to both, in parallel"));
    }
    out
}
