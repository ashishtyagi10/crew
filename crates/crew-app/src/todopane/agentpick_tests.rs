use super::{pick, Agent, Facts, Markers};

const ALL: [Agent; 3] = [Agent::Claude, Agent::Codex, Agent::Opencode];

fn facts<'a>(
    on_path: &'a [Agent],
    signed_in: &'a [&'a str],
    serving: Option<&'a str>,
) -> Facts<'a> {
    Facts {
        on_path,
        signed_in,
        serving,
    }
}

#[test]
fn an_assignee_that_names_an_agent_wins_and_a_person_does_not() {
    let f = facts(&ALL, &[], None);
    assert_eq!(
        pick(Some("opencode"), Markers::default(), &f),
        Ok((Agent::Opencode, "#opencode".into()))
    );
    assert_eq!(Agent::from_assignee("Claude-Code"), Some(Agent::Claude));
    assert_eq!(Agent::from_assignee("priya"), None, "a person, as before");
    // A person on the item leaves the ladder to the other rungs.
    assert_eq!(
        pick(Some("priya"), Markers::default(), &f).map(|(a, _)| a),
        Ok(Agent::Claude)
    );
}

#[test]
fn a_named_agent_that_is_not_installed_is_a_refusal_not_a_fallthrough() {
    let f = facts(&[Agent::Claude], &[], None);
    assert_eq!(
        pick(Some("codex"), Markers::default(), &f),
        Err("codex is not on PATH".into())
    );
    // Smith is crew itself and is always there.
    assert_eq!(
        pick(Some("smith"), Markers::default(), &facts(&[], &[], None)),
        Ok((Agent::Smith, "#smith".into()))
    );
}

#[test]
fn a_marker_file_votes_and_two_votes_tie() {
    let f = facts(&ALL, &["claude-code"], None);
    let claude = Markers {
        claude: true,
        ..Default::default()
    };
    assert_eq!(
        pick(None, claude, &f),
        Ok((Agent::Claude, "CLAUDE.md, signed in".into()))
    );
    let agents_only = Markers {
        agents_md: true,
        ..Default::default()
    };
    assert_eq!(
        pick(None, agents_only, &f),
        Ok((Agent::Codex, "AGENTS.md".into()))
    );
    let both = Markers {
        claude: true,
        opencode: true,
        agents_md: true,
    };
    // A tie falls through to the sign-in rung.
    assert_eq!(
        pick(None, both, &f),
        Ok((Agent::Claude, "signed in".into()))
    );
    // A marker for a CLI that is not installed is skipped, not refused.
    let oc = Markers {
        opencode: true,
        ..Default::default()
    };
    assert_eq!(
        pick(None, oc, &facts(&[Agent::Codex], &[], None)),
        Ok((Agent::Codex, "on PATH".into()))
    );
}

#[test]
fn signed_in_beats_installed_and_serving_breaks_the_tie() {
    let f = facts(&ALL, &["codex", "claude-code"], Some("codex"));
    assert_eq!(
        pick(None, Markers::default(), &f),
        Ok((Agent::Codex, "signed in, serving".into()))
    );
    let f = facts(&ALL, &["codex"], None);
    assert_eq!(
        pick(None, Markers::default(), &f).map(|(a, _)| a),
        Ok(Agent::Codex)
    );
    let f = facts(&[Agent::Opencode, Agent::Codex], &[], None);
    assert_eq!(
        pick(None, Markers::default(), &f),
        Ok((Agent::Codex, "on PATH".into())),
        "no sign-in known: the CLI order decides"
    );
}

#[test]
fn nothing_on_path_falls_to_smith() {
    assert_eq!(
        pick(
            None,
            Markers::default(),
            &facts(&[], &["claude-code"], None)
        ),
        Ok((Agent::Smith, "no CLI on PATH".into()))
    );
}

#[test]
fn each_cli_takes_the_task_as_its_opening_prompt() {
    assert_eq!(
        Agent::Claude.invocation("fix it"),
        Some(("claude", vec!["fix it".into()]))
    );
    assert_eq!(
        Agent::Opencode.invocation("fix it"),
        Some(("opencode", vec!["--prompt".into(), "fix it".into()]))
    );
    assert_eq!(Agent::Smith.invocation("fix it"), None);
}

#[test]
fn markers_read_the_project_root() {
    let t = tempfile::tempdir().unwrap();
    assert_eq!(Markers::at(t.path()), Markers::default());
    std::fs::create_dir(t.path().join(".claude")).unwrap();
    std::fs::write(t.path().join("AGENTS.md"), "").unwrap();
    assert_eq!(
        Markers::at(t.path()),
        Markers {
            claude: true,
            opencode: false,
            agents_md: true
        }
    );
}
