use super::HELP;

/// The lines that name a construct, in order — the mechanical verb list.
fn construct_lines() -> Vec<String> {
    HELP.lines()
        .filter_map(|l| {
            let rest = l.trim_start().strip_prefix('/')?;
            Some(
                rest.chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect(),
            )
        })
        .collect()
}

/// `/help` describes what the brain DECIDES, not only the verbs: the shape,
/// the size, who, the check, the tools and skills, the memory of the last
/// turns, and that an approved plan runs as the swarm.
#[test]
fn help_says_what_agent_smith_decides() {
    let para = HELP
        .lines()
        .find(|l| l.trim_start().starts_with("agent smith decides:"))
        .expect("the paragraph is present");
    for word in [
        "shape",
        "routing line",
        "rounds",
        "which agents",
        "verifies",
        "tools",
        "skills",
        "last six turns",
        "approved plan runs as the swarm",
        "context line",
    ] {
        assert!(
            para.contains(word),
            "the paragraph never says {word:?}: {para}"
        );
    }
}

/// The verb list is unchanged by the paragraph: the same constructs, in the
/// same order, and the paragraph is prose — it starts with no `/`, so it can
/// neither advertise a construct nor be read as one's summary.
#[test]
fn the_construct_list_is_intact_around_the_paragraph() {
    assert_eq!(
        construct_lines(),
        [
            "help", "model", "model", "model", "model", "logout", "restore", "diff", "doctor",
            "reload", "stop"
        ]
    );
    assert_eq!(super::super::construct_summary("help"), Some("this list"));
}
