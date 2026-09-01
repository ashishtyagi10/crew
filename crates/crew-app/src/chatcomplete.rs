//! Tab-completion for the crew composer: `@ag<Tab>` completes agent names
//! (including the segment after a `+` in multi-target selectors) and
//! `/lo<Tab>` completes construct names. Pure string-in/string-out so it's
//! trivially testable.
pub(crate) use crate::completefuzzy::*;
use crew_plugin::AgentInfo;

/// Every composer slash action: broker constructs plus the pane-local
/// `/export`, `/theme`, and `/exit` (see `chatexport` / `chattheme` /
/// `chat`). Folding the transcript is automatic (`ChatPane::push_capped`).
pub(crate) const CONSTRUCTS: [&str; 12] = [
    "/help", "/model", "/login", "/logout", "/restore", "/diff", "/doctor", "/reload", "/stop",
    "/export", "/theme", "/exit",
];

/// Hints that belong to the PANE rather than to the broker, and so are written
/// here instead of derived: the three pane-local constructs the broker has
/// never heard of, plus the two where standing in the pane changes what is
/// worth saying — "this list" reads as nothing in a palette that IS the list.
///
/// Being a short, declared list is the point. Everything absent from it shows
/// the broker's own sentence, so a hint cannot quietly contradict the command
/// it labels — which is exactly what `/goal` once did.
const PANE_WORDS: &[(&str, &str)] = &[
    ("/help", "list the constructs"),
    (
        "/model",
        "the roster and each agent's model (set one: /model <agent> <model>)",
    ),
    ("/export", "export the transcript"),
    ("/theme", "list or switch the color theme"),
    ("/exit", "close this pane"),
];

/// One-line description for each construct, shown as the dim hint in the
/// composer's slash palette. "" for anything the palette does not offer — a
/// construct that is deliberately withheld (`/approve`) or gone entirely has
/// no hint, because there is no row to hint at.
pub(crate) fn describe(construct: &str) -> &'static str {
    if !CONSTRUCTS.contains(&construct) {
        return "";
    }
    if let Some((_, words)) = PANE_WORDS.iter().find(|(c, _)| *c == construct) {
        return words;
    }
    crew_plugin::construct_summary(construct.trim_start_matches('/')).unwrap_or("")
}

/// Complete `input`'s leading token. Returns the new input when something
/// completed (unique match, or extended to the candidates' common prefix).
pub(crate) fn complete(input: &str, agents: &[AgentInfo]) -> Option<String> {
    // Only the first token completes, and only while the cursor is inside it
    // (the composer has no mid-line cursor — input is append-only).
    if input.contains(char::is_whitespace) {
        return None;
    }
    if let Some(rest) = input.strip_prefix('@') {
        // Complete the segment after the last '+' (multi-target selectors).
        let (done, part) = match rest.rfind('+') {
            Some(i) => (&rest[..=i], &rest[i + 1..]),
            None => ("", rest),
        };
        let names: Vec<&str> = agents.iter().map(|a| a.name.as_str()).collect();
        let (ext, unique) = match extend(part, &names) {
            Some(pair) => pair,
            // Prefix matching found nothing — fall back to a fuzzy (opencode-
            // style subsequence) match, but only if it's unambiguous.
            None => (fuzzy_unique(part, &names)?.to_string(), true),
        };
        let tail = if unique && done.is_empty() { " " } else { "" };
        return Some(format!("@{done}{ext}{tail}"));
    }
    if input.starts_with('/') {
        let (ext, unique) = match extend(input, &CONSTRUCTS) {
            Some(pair) => pair,
            None => (fuzzy_unique(input, &CONSTRUCTS)?.to_string(), true),
        };
        let tail = if unique { " " } else { "" };
        return Some(format!("{ext}{tail}"));
    }
    None
}

#[cfg(test)]
#[path = "chatcomplete_tests.rs"]
mod tests;
#[cfg(test)]
mod drift {
    use super::CONSTRUCTS;

    /// Constructs the APP answers by itself — the broker has never heard of
    /// them, so their absence from its router is correct, not drift.
    const APP_LOCAL: &[&str] = &["/export", "/theme", "/exit"];

    /// Every command the broker answers is offered. (The old "withheld"
    /// class — `/approve`/`/reject`, sent by the pane's enter/esc — retired
    /// as commands: the pane now sends the bare words the broker's plan gate
    /// matches deterministically.) `/reload` was once neither offered nor
    /// withheld, for eleven releases: two lists existed and nothing compared
    /// them, so a working command was simply invisible.
    #[test]
    fn every_broker_construct_is_offered() {
        for c in crew_plugin::broker_constructs() {
            let slashed = format!("/{c}");
            assert!(
                CONSTRUCTS.contains(&slashed.as_str()),
                "{slashed} is a broker command the palette never offers"
            );
        }
    }

    /// …and nothing is offered that nobody answers. A name that completes but
    /// does not route is worse than one that never existed.
    #[test]
    fn nothing_offered_is_unanswerable() {
        for c in CONSTRUCTS {
            let bare = c.trim_start_matches('/');
            assert!(
                crew_plugin::broker_constructs().contains(&bare) || APP_LOCAL.contains(&c),
                "{c} routes nowhere"
            );
        }
    }

    /// Every offered construct explains itself; a blank hint is a row the
    /// user has to guess at.
    #[test]
    fn every_construct_describes_itself() {
        for c in CONSTRUCTS {
            assert!(!super::describe(c).is_empty(), "{c} has no description");
        }
    }
}

#[cfg(test)]
#[path = "chatcomplete_env_tests.rs"]
mod env_drift;

#[cfg(test)]
mod doc_drift {
    /// Construct names the user-facing docs may mention. Anything in a
    /// `` `/name` `` code span has to be a construct the broker answers, a
    /// command the app answers, or something on this list — paths and URLs
    /// are not commands, and a doc that documents a deleted one is worse than
    /// a doc that says nothing.
    ///
    /// Ten constructs went in one night and both README.md and docs/CREW.md
    /// still described all ten the next morning. Prose drifts exactly like
    /// the code lists did; it just has nobody to fail.
    pub(super) const DOCS: &[&str] = &["../../README.md", "../../docs/CREW.md"];

    /// Words that mark a line as HISTORY rather than instruction. Docs
    /// legitimately say "`/edit` and `/open` were dropped"; a guard that
    /// cannot tell that from "use `/edit`" would force the prose to stop
    /// explaining itself, which is a worse outcome than the drift.
    const HISTORICAL: &[&str] = &[
        "dropped",
        "removed",
        "no longer",
        "gone",
        "replaced",
        "used to",
        "merged into",
        "retired",
    ];

    fn documented_constructs(src: &str) -> Vec<String> {
        let mut out = Vec::new();
        // Line by line, so a historical mention excuses only its own line.
        let src: String = src
            .lines()
            .filter(|l| {
                let low = l.to_lowercase();
                !HISTORICAL.iter().any(|w| low.contains(w))
            })
            .collect::<Vec<_>>()
            .join("\n");
        let bytes: Vec<char> = src.chars().collect();
        let mut i = 0;
        while i < bytes.len() {
            // Only inside a backtick code span, and only `/name` at its start.
            if bytes[i] == '`' {
                let rest: String = bytes[i + 1..].iter().take(40).collect();
                if let Some(after) = rest.strip_prefix('/') {
                    let name: String = after
                        .chars()
                        .take_while(|c| c.is_ascii_lowercase())
                        .collect();
                    // A path continues past the name (`/skills/`), a command
                    // is followed by a backtick, a space or an argument.
                    let next = after.chars().nth(name.len());
                    if !name.is_empty() && !matches!(next, Some('/') | Some('.')) {
                        out.push(name);
                    }
                }
            }
            i += 1;
        }
        out.sort();
        out.dedup();
        out
    }

    #[test]
    fn the_docs_do_not_describe_constructs_that_no_longer_exist() {
        let app_local = ["export", "theme", "exit", "shell", "run", "crew"];
        for rel in DOCS {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
            let Ok(src) = std::fs::read_to_string(&path) else {
                continue; // not shipped in every build context
            };
            for name in documented_constructs(&src) {
                let slashed = format!("/{name}");
                // An alias counts as answered when the router expands it to
                // something that is — resolved through the router's own table
                // rather than a copy of it here.
                let expanded = crew_plugin::expand_alias(&slashed);
                let bare = expanded.trim_start_matches('/');
                let known = crew_plugin::broker_constructs().contains(&bare)
                    || super::CONSTRUCTS.contains(&expanded.as_str())
                    || crate::cmddefs::commands().any(|c| c.name == expanded)
                    || app_local.contains(&bare);
                assert!(
                    known,
                    "{} documents `{slashed}`, which nothing answers",
                    path.display()
                );
            }
        }
    }

    /// Command-bar commands the manual deliberately does not carry: session
    /// plumbing and aliases whose target is documented instead. Declared, so
    /// "not worth a paragraph" stays distinguishable from "nobody wrote one".
    const UNDOCUMENTED_BY_CHOICE: &[&str] = &["/crew"];

    /// Every command the command-bar palette offers is described somewhere a
    /// user can read. `/crt` and `/weight` shipped as working, palette-listed
    /// commands with no mention in either page — the `/reload` shape again, one
    /// list further out.
    #[test]
    fn every_command_bar_command_is_documented() {
        let mut docs = String::new();
        for rel in DOCS {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
            if let Ok(s) = std::fs::read_to_string(&path) {
                docs.push_str(&s);
            }
        }
        if docs.is_empty() {
            return; // docs not shipped in this build context
        }
        for c in crate::cmddefs::commands() {
            assert!(
                docs.contains(c.name) || UNDOCUMENTED_BY_CHOICE.contains(&c.name),
                "{} is in the command palette and in no doc",
                c.name
            );
        }
    }

    /// A heading's GitHub anchor: lowercased, punctuation dropped, spaces
    /// hyphenated. `## Multi-agent relay (`/smith`, alias `/crew`)` becomes
    /// `multi-agent-relay-smith-alias-crew`.
    fn anchor_of(heading: &str) -> String {
        heading
            .trim()
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("-")
    }

    /// An in-document link must land on a heading that exists. Renaming a
    /// section silently breaks every link to it — `#multi-agent-relay-crew`
    /// outlived the `/smith` rename by months, and a link that goes nowhere
    /// reads as a missing feature rather than a stale anchor. (Caught a second
    /// one written during this very loop, which is why it is a test.)
    #[test]
    fn internal_doc_links_land_on_a_real_heading() {
        for rel in DOCS {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
            let Ok(src) = std::fs::read_to_string(&path) else {
                continue; // not shipped in every build context
            };
            let anchors: Vec<String> = src
                .lines()
                .filter_map(|l| l.trim_start().strip_prefix('#'))
                .map(|h| anchor_of(h.trim_start_matches('#')))
                .collect();
            for target in internal_link_targets(&src) {
                assert!(
                    anchors.contains(&target),
                    "{} links to #{target}, which is no heading in it",
                    path.display()
                );
            }
        }
    }

    /// Every `](#target)` in `src`.
    fn internal_link_targets(src: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = src;
        while let Some(i) = rest.find("](#") {
            rest = &rest[i + 3..];
            if let Some(end) = rest.find(')') {
                out.push(rest[..end].to_string());
            }
        }
        out
    }
}
