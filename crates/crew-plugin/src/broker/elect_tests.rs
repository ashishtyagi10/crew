use super::*;

fn info(name: &str, role: &str) -> crate::AgentInfo {
    crate::AgentInfo {
        name: name.into(),
        role: role.into(),
        model: String::new(),
    }
}

fn trio() -> Vec<crate::AgentInfo> {
    vec![
        info("travel-advisor", "trips, itineraries"),
        info("quality-auditor", "review, critique"),
        info("release-scribe", "drafts release notes, writing"),
    ]
}

#[test]
fn parse_agent_reads_the_grammar_and_returns_the_canonical_name() {
    let agents = trio();
    assert_eq!(
        parse_agent("AGENT: quality-auditor", &agents).as_deref(),
        Some("quality-auditor")
    );
    // Case-insensitive on both the head and the name; trailing punctuation
    // and prose after the first line tolerated — same conservatism as the
    // intent router's `SHAPE:` grammar.
    assert_eq!(
        parse_agent("agent: Quality-Auditor.\nbecause it reviews", &agents).as_deref(),
        Some("quality-auditor")
    );
}

#[test]
fn parse_agent_rejects_off_grammar_and_unknown_names() {
    let agents = trio();
    for bad in [
        "",
        "quality-auditor",
        "I'd pick quality-auditor",
        "AGENT:",
        "AGENT: nobody-here",
        "AGENTS: quality-auditor",
    ] {
        assert_eq!(parse_agent(bad, &agents), None, "{bad:?}");
    }
}

#[test]
fn elect_with_uses_the_models_choice() {
    let call = |_: &str| Ok("AGENT: release-scribe".to_string());
    assert_eq!(
        elect_with("write a commit message", &trio(), None, Some(&call)),
        "release-scribe"
    );
}

#[test]
fn elect_with_falls_back_on_off_grammar_error_or_no_classifier() {
    let agents = trio();
    let off = |_: &str| Ok("hmm, hard to say".to_string());
    let err = |_: &str| Err("boom".to_string());
    // The deterministic fallback is the roster's first agent — the keyless/
    // mock path must stay exactly as predictable as it was.
    assert_eq!(
        elect_with("judge the result", &agents, None, Some(&off)),
        "travel-advisor"
    );
    assert_eq!(
        elect_with("judge the result", &agents, None, Some(&err)),
        "travel-advisor"
    );
    assert_eq!(
        elect_with("judge the result", &agents, None, None),
        "travel-advisor"
    );
}

#[test]
fn the_excluded_worker_is_never_elected_while_an_alternative_exists() {
    let agents = trio();
    // The model names the worker itself — invalid, so the deterministic
    // fallback (first non-worker) is used instead.
    let own_homework = |_: &str| Ok("AGENT: travel-advisor".to_string());
    assert_eq!(
        elect_with(
            "judge the result",
            &agents,
            Some("travel-advisor"),
            Some(&own_homework)
        ),
        "quality-auditor"
    );
    // …and the fallback itself skips the worker too.
    assert_eq!(
        elect_with("judge the result", &agents, Some("travel-advisor"), None),
        "quality-auditor"
    );
    // A single-agent roster has nobody else: the worker judges itself.
    let solo = vec![info("solo", "")];
    assert_eq!(elect_with("judge", &solo, Some("solo"), None), "solo");
}

#[test]
fn the_prompt_carries_the_purpose_the_roster_and_the_grammar() {
    let seen = std::sync::Mutex::new(String::new());
    let call = |p: &str| {
        *seen.lock().unwrap() = p.to_string();
        Ok("AGENT: quality-auditor".to_string())
    };
    elect_with(
        "judge whether the goal is met",
        &trio(),
        Some("travel-advisor"),
        Some(&call),
    );
    let p = seen.lock().unwrap();
    assert!(p.contains("judge whether the goal is met"), "{p}");
    for name in ["travel-advisor", "quality-auditor", "release-scribe"] {
        assert!(p.contains(name), "roster missing {name}: {p}");
    }
    assert!(p.contains("review, critique"), "roles ride along: {p}");
    assert!(p.contains("AGENT: <name>"), "{p}");
    assert!(
        p.contains("not travel-advisor"),
        "the exclusion is stated: {p}"
    );
}
