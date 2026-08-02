//! Tab-completion for the crew composer: `@ag<Tab>` completes agent names
//! (including the segment after a `+` in multi-target selectors) and
//! `/lo<Tab>` completes construct names. Pure string-in/string-out so it's
//! trivially testable.
use crew_plugin::AgentInfo;

/// Every composer slash action: broker constructs plus the pane-local
/// `/export`, `/theme`, and `/exit` (see `chatexport` / `chattheme` /
/// `chat`). Folding the transcript is automatic (`ChatPane::push_capped`).
pub(crate) const CONSTRUCTS: [&str; 10] = [
    "/help", "/model", "/restore", "/diff", "/doctor", "/reload", "/stop", "/export", "/theme",
    "/exit",
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

/// Extend `prefix` against `candidates`: the full name when exactly one
/// matches (`(name, true)`), else the longest common prefix when it grows the
/// input (`(lcp, false)`). Case-insensitive; `None` when nothing matches or
/// nothing would change.
fn extend(prefix: &str, candidates: &[&str]) -> Option<(String, bool)> {
    let low = prefix.to_lowercase();
    let hits: Vec<&&str> = candidates
        .iter()
        .filter(|c| c.to_lowercase().starts_with(&low))
        .collect();
    match hits.as_slice() {
        [] => None,
        [one] => Some((one.to_string(), true)),
        many => {
            let first = many[0].to_lowercase();
            let mut lcp = first.len();
            for c in many.iter().skip(1) {
                let c = c.to_lowercase();
                lcp = first
                    .chars()
                    .zip(c.chars())
                    .take(lcp)
                    .take_while(|(a, b)| a == b)
                    .count();
            }
            (lcp > prefix.len()).then(|| (first[..lcp].to_string(), false))
        }
    }
}

/// Case-insensitive subsequence match: is every char of `needle` found in
/// `hay` in order? (`"gl"` matches `"goal"`, `"pnr"` matches `"planner"`.)
/// Case-folds and delegates to `crate::suggest`'s identical (already
/// case-normalized-by-caller) helper.
fn is_subsequence(needle: &str, hay: &str) -> bool {
    crate::suggest::is_subsequence(&needle.to_lowercase(), &hay.to_lowercase())
}

/// The single candidate that fuzzy-matches `needle`, or `None` if zero or
/// more than one do.
fn fuzzy_unique<'a>(needle: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let mut hits = candidates.iter().filter(|c| is_subsequence(needle, c));
    let first = *hits.next()?;
    match hits.next() {
        Some(_) => None,
        None => Some(first),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agents(names: &[&str]) -> Vec<AgentInfo> {
        names
            .iter()
            .map(|n| AgentInfo {
                name: (*n).into(),
                role: String::new(),
                model: String::new(),
            })
            .collect()
    }

    #[test]
    fn completes_a_unique_agent_with_trailing_space() {
        let a = agents(&["planner", "coder", "reviewer"]);
        assert_eq!(complete("@pl", &a).unwrap(), "@planner ");
        assert_eq!(complete("@CO", &a).unwrap(), "@coder ");
    }

    #[test]
    fn completes_the_segment_after_a_plus() {
        let a = agents(&["planner", "coder", "reviewer"]);
        assert_eq!(complete("@planner+co", &a).unwrap(), "@planner+coder");
    }

    #[test]
    fn ambiguous_prefix_extends_to_common_prefix() {
        let a = agents(&["planner", "plotter"]);
        assert_eq!(complete("@p", &a).unwrap(), "@pl");
        // Already at the common prefix → nothing to add.
        assert_eq!(complete("@pl", &a), None);
    }

    #[test]
    fn completes_constructs() {
        // `/goal` and `/loop` retired (the intent router answers the plain
        // phrasing), so their prefixes fall to the survivors: nothing starts
        // with "go" any more, and `/lo` fuzzy-matches /reload.
        assert_eq!(complete("/go", &[]), None);
        assert_eq!(complete("/lo", &[]).unwrap(), "/reload ");
        // `/standup` retired too, so '/st' now uniquely names /stop — every
        // retirement buys back a prefix — and '/sta' matches nothing at all.
        assert_eq!(complete("/st", &[]).unwrap(), "/stop ");
        assert_eq!(complete("/sta", &[]), None);
        // `/memory` and `/mcp` gone: '/m' is /model's alone now.
        assert_eq!(complete("/m", &[]).unwrap(), "/model ");
    }

    #[test]
    fn completes_and_describes_diff() {
        assert_eq!(complete("/di", &[]).unwrap(), "/diff ");
        // Not the palette's own sentence any more — the broker's, verbatim.
        assert_eq!(
            describe("/diff"),
            "everything different from the last commit, new files included"
        );
    }

    /// The hint for a derived construct IS the broker's `/help` line, so the
    /// two cannot disagree. `/goal` (now retired) was the regression: the
    /// palette called it "set the crew's shared goal" — a feature that
    /// existed nowhere — while the broker looped rounds until a judge ruled
    /// the goal met.
    #[test]
    fn derived_hints_are_the_brokers_own_words() {
        for c in CONSTRUCTS {
            if PANE_WORDS.iter().any(|(p, _)| *p == c) {
                continue;
            }
            let bare = c.trim_start_matches('/');
            assert_eq!(
                describe(c),
                crew_plugin::construct_summary(bare).unwrap_or(""),
                "{c}'s hint is not the broker's own line"
            );
        }
    }

    /// Every pane-written hint must name a construct the palette actually
    /// offers, or it is dead text nobody will ever see — and the next reader
    /// will trust it anyway.
    #[test]
    fn pane_words_only_override_offered_constructs() {
        for (c, _) in PANE_WORDS {
            assert!(CONSTRUCTS.contains(c), "{c} is overridden but not offered");
        }
    }

    /// Deleted constructs must leave the palette entirely — a name that still
    /// completes but no longer routes is worse than one that never existed.
    #[test]
    fn deleted_constructs_do_not_complete() {
        for gone in [
            "/cwd",
            "/agents",
            "/tasks",
            "/status",
            "/checkpoint",
            "/approve",
            "/reject",
            "/commit",
            "/review",
            "/standup",
            "/resume",
            "/goal",
            "/plan",
            "/skill",
            "/memory",
            "/mcp",
        ] {
            assert!(!CONSTRUCTS.contains(&gone), "{gone} still listed");
            assert_eq!(describe(gone), "", "{gone} still described");
        }
        assert_eq!(complete("/cw", &[]), None);
    }

    #[test]
    fn ignores_mid_sentence_and_plain_text() {
        let a = agents(&["planner"]);
        assert_eq!(complete("@planner do the", &a), None);
        assert_eq!(complete("hello", &a), None);
        assert_eq!(complete("", &a), None);
        assert_eq!(complete("@ghost", &a), None);
    }

    #[test]
    fn fuzzy_fallback_completes_a_unique_subsequence_match() {
        assert_eq!(complete("/hp", &[]).unwrap(), "/help ");
        let a = agents(&["planner", "coder", "reviewer"]);
        assert_eq!(complete("@pnr", &a).unwrap(), "@planner ");
    }

    #[test]
    fn fuzzy_fallback_is_none_when_ambiguous() {
        let a = agents(&["planner", "cleaner"]);
        // "an" is a subsequence of both "planner" and "cleaner".
        assert_eq!(complete("@an", &a), None);
    }

    #[test]
    fn prefix_match_still_wins_over_fuzzy() {
        // "/re" is a shared prefix (/restore, /reload) already at its common
        // prefix, and a fuzzy subsequence of more — stays ambiguous.
        assert_eq!(complete("/re", &[]), None);
    }

    #[test]
    fn is_subsequence_cases() {
        assert!(is_subsequence("hp", "help"));
        assert!(is_subsequence("pnr", "planner"));
        assert!(is_subsequence("", "anything"));
        assert!(!is_subsequence("xyz", "goal"));
        assert!(!is_subsequence("lg", "goal")); // wrong order
    }
}

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
                    || crate::cmddefs::COMMANDS.iter().any(|c| c.name == expanded)
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
        for c in crate::cmddefs::COMMANDS {
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
