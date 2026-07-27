use super::*;
use crate::chatkeys::ChatInput;
use crew_plugin::AgentInfo;

fn agents() -> Vec<AgentInfo> {
    // AgentInfo does NOT derive Default — construct all three fields.
    ["planner", "coder"]
        .iter()
        .map(|n| AgentInfo {
            name: n.to_string(),
            role: "role".into(),
            model: String::new(),
        })
        .collect()
}

#[test]
fn pending_palette_detects_leading_slash_and_agent() {
    assert_eq!(pending_palette("/mod"), Some((Kind::Slash, "mod")));
    assert_eq!(pending_palette("@co"), Some((Kind::Agent, "co")));
    assert_eq!(pending_palette("@a+co"), Some((Kind::Agent, "co"))); // segment after '+'
    assert_eq!(pending_palette("@planner"), Some((Kind::Agent, "planner")));
    assert_eq!(pending_palette("hey @co"), None); // non-leading → file mention's job
    assert_eq!(pending_palette("/theme x"), None); // token ended (no arg picker)
    assert_eq!(pending_palette("plain"), None);
    assert_eq!(pending_palette(""), None);
}

#[test]
fn accept_replaces_leading_token_preserving_multi_target() {
    assert_eq!(accept("/mod", Kind::Slash, "/model"), "/model ");
    assert_eq!(accept("@co", Kind::Agent, "coder"), "@coder ");
    assert_eq!(accept("@a+co", Kind::Agent, "coder"), "@a+coder ");
}

fn agent_entries() -> Vec<crate::chatmention::MentionEntry> {
    agents()
        .iter()
        .map(|a| crate::chatmention::MentionEntry::Agent {
            name: a.name.clone(),
            role: a.role.clone(),
        })
        .collect()
}

#[test]
fn after_edit_opens_refilters_and_closes() {
    let mut p = None;
    after_edit(&mut p, "@", None, agent_entries);
    assert_eq!(p.as_ref().unwrap().items.len(), 2);
    assert_eq!(p.as_ref().unwrap().kind, Kind::Agent);
    after_edit(&mut p, "@co", None, agent_entries);
    assert_eq!(p.as_ref().unwrap().items.len(), 1); // only coder
    after_edit(&mut p, "@zzz", None, agent_entries);
    assert!(p.is_none()); // no match closes
    after_edit(&mut p, "/mo", None, Vec::new);
    assert_eq!(p.as_ref().unwrap().kind, Kind::Slash);
    assert!(p.as_ref().unwrap().items.iter().any(|i| i.fill == "/model"));
    after_edit(&mut p, "hey", None, agent_entries);
    assert!(p.is_none()); // no leading selector
}

#[test]
fn leading_at_offers_agents_skills_and_files_in_order() {
    let mut p = None;
    let mut entries: Vec<crate::chatmention::MentionEntry> = vec![
        crate::chatmention::MentionEntry::Agent {
            name: "reviewer".into(),
            role: "r".into(),
        },
        crate::chatmention::MentionEntry::Skill {
            name: "review".into(),
            desc: "d".into(),
        },
        crate::chatmention::MentionEntry::File("review.md".into()),
    ];
    after_edit(&mut p, "@rev", None, || entries.clone());
    let items = &p.as_ref().unwrap().items;
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["@reviewer", "@skill:review", "@review.md"]);
    assert_eq!(items[1].fill, "skill:review");
    assert_eq!(items[2].fill, "review.md");
    // narrowing refilters without rescanning
    entries.clear();
    after_edit(&mut p, "@revi", None, || {
        unreachable!("no rescan while open")
    });
    assert!(p.is_some());
}

#[test]
fn multi_target_plus_offers_agents_only() {
    let mut p = None;
    let entries = vec![
        crate::chatmention::MentionEntry::Agent {
            name: "coder".into(),
            role: "c".into(),
        },
        crate::chatmention::MentionEntry::File("coder.md".into()),
    ];
    after_edit(&mut p, "@planner+co", None, || entries.clone());
    let labels: Vec<&str> = p
        .as_ref()
        .unwrap()
        .items
        .iter()
        .map(|i| i.label.as_str())
        .collect();
    assert_eq!(labels, vec!["@coder"]);
}

#[test]
fn pending_palette_detects_the_model_arg_phase() {
    assert_eq!(pending_palette("/model "), Some((Kind::Model, "")));
    assert_eq!(pending_palette("/model son"), Some((Kind::Model, "son")));
    assert_eq!(pending_palette("/model  son"), Some((Kind::Model, "son")));
    // Two argument tokens = the freeform per-agent / explicit-all form.
    assert_eq!(pending_palette("/model all qwen-max"), None);
    assert_eq!(pending_palette("/model coder qwen"), None);
    // Still the plain slash palette before the space.
    assert_eq!(pending_palette("/model"), Some((Kind::Slash, "model")));
    // Other commands keep their old behaviour.
    assert_eq!(pending_palette("/theme dark"), None);
}

#[test]
fn the_m_alias_opens_the_same_model_picker() {
    // `/m` is the broker's built-in alias for `/model` (`expand_alias`); it
    // must open the same picker.
    assert_eq!(pending_palette("/m "), Some((Kind::Model, "")));
    assert_eq!(pending_palette("/m son"), Some((Kind::Model, "son")));
    // Two argument tokens: freeform passthrough, same as the long form.
    assert_eq!(pending_palette("/m coder qwen"), None);
    // A command that merely starts with "/m" must not false-match.
    assert_eq!(pending_palette("/memory foo"), None);
}

#[test]
fn model_rows_accept_into_a_full_broker_command_and_submit() {
    let mut p = None;
    after_edit(&mut p, "/model son", None, Vec::new);
    let open = p.as_ref().expect("model picker opens");
    assert_eq!(open.kind, Kind::Model);
    assert!(open.items.iter().any(|i| i.header)); // grouped
    assert!(!open.items[open.sel].header); // selection never starts on a header

    let mut input = "/model son".to_string();
    let key = popup_key(&mut p, &mut input, &ChatInput::Enter);
    assert!(matches!(key, PaletteKey::Submit));
    assert!(input.starts_with("/model all "), "{input}");
    assert!(p.is_none()); // accepting closes
}

#[test]
fn popup_key_navigates_accepts_and_closes() {
    let mut p = None;
    after_edit(&mut p, "@", None, agent_entries);
    let mut input = "@".to_string();
    assert!(matches!(
        popup_key(&mut p, &mut input, &ChatInput::Down),
        PaletteKey::Consumed
    ));
    // Filling a row without running it reports `Accepted`, not `Consumed`:
    // the input changed, so the caller has to re-sync the palette to it.
    assert!(matches!(
        popup_key(&mut p, &mut input, &ChatInput::Enter),
        PaletteKey::Accepted
    ));
    assert!(input.starts_with('@') && input.ends_with(' '));
    assert!(p.is_none());
    // Esc closes the popup, not the pane.
    after_edit(&mut p, "/", None, Vec::new);
    assert!(matches!(
        popup_key(&mut p, &mut input, &ChatInput::Close),
        PaletteKey::Consumed
    ));
    assert!(p.is_none());
    // Closed popup forwards.
    let mut none: Option<PaletteState> = None;
    assert!(matches!(
        popup_key(&mut none, &mut input, &ChatInput::Enter),
        PaletteKey::Forward
    ));
}

fn agent(name: &str, model: &str) -> crew_plugin::AgentInfo {
    crew_plugin::AgentInfo {
        name: name.to_string(),
        role: "role".into(),
        model: model.to_string(),
    }
}

#[test]
fn shared_model_agrees_when_every_agent_reports_the_same_model() {
    let agents = vec![agent("planner", "qwen-max"), agent("coder", "qwen-max")];
    assert_eq!(shared_model(&agents), Some("qwen-max".to_string()));
}

#[test]
fn shared_model_none_when_agents_disagree() {
    let agents = vec![agent("planner", "qwen-max"), agent("coder", "sonnet")];
    assert_eq!(shared_model(&agents), None);
}

#[test]
fn shared_model_none_for_an_empty_roster() {
    let agents: Vec<crew_plugin::AgentInfo> = Vec::new();
    assert_eq!(shared_model(&agents), None);
}

#[test]
fn shared_model_ignores_empty_entries_when_the_rest_agree() {
    let agents = vec![
        agent("planner", ""),
        agent("coder", "qwen-max"),
        agent("reviewer", "qwen-max"),
    ];
    assert_eq!(shared_model(&agents), Some("qwen-max".to_string()));
}

#[test]
fn accepting_a_row_that_needs_a_key_asks_for_it_instead_of_running() {
    let mut input = "/model claude".to_string();
    let before = input.clone();
    let mut palette = Some(PaletteState {
        kind: Kind::Model,
        items: vec![MenuItem {
            label: "Claude Opus".into(),
            desc: "needs ANTHROPIC_API_KEY".into(),
            fill: "claude-opus-5".into(),
            submit: true,
            header: false,
            dim: true,
            needs: Some("ANTHROPIC_API_KEY".into()),
        }],
        sel: 0,
        entries: Vec::new(),
        touched: true,
    });
    match popup_key(&mut palette, &mut input, &ChatInput::Enter) {
        PaletteKey::NeedsKey(var) => assert_eq!(var, "ANTHROPIC_API_KEY"),
        _ => panic!("a keyless row must not run"),
    }
    assert_eq!(
        input, before,
        "the model must not be chosen until it can run"
    );
    assert!(
        palette.is_none(),
        "the palette closes to make room for the prompt"
    );
}
