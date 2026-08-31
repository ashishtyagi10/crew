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
    // phrasing); `/login`/`/logout` arrived: `/go` fuzzy-matches /logout
    // uniquely (g-o in order — /login has no 'o' after its 'g'), and
    // `/lo` prefix-extends to their common "/log".
    assert_eq!(complete("/go", &[]).unwrap(), "/logout ");
    assert_eq!(complete("/lo", &[]).unwrap(), "/log");
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
