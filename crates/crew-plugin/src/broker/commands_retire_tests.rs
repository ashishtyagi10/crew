//! Retired commands: everything that was a prompt-in-a-trenchcoat left the
//! construct surface (its capability now answers plain language through the
//! intent router), and the router must say so helpfully — instantly, with the
//! replacement phrasing, never a silent reinterpretation or a generic
//! did-you-mean.
use super::*;

fn run(text: &str) -> Vec<PluginEvent> {
    let mut session = Session::new();
    let mut out = Vec::new();
    handle(
        &mut session,
        text,
        &crate::broker::tick::noop_tick_emit(),
        &mut |ev| {
            out.push(ev);
            Ok(())
        },
    )
    .unwrap();
    out
}

fn text_of(ev: &PluginEvent) -> &str {
    match ev {
        PluginEvent::Message { text, .. } => text,
        _ => "",
    }
}

/// Every retired slash form, with a representative argument where one was
/// customary. One list, shared by the hint/inline tests below.
const RETIRED_FORMS: &[&str] = &[
    "/fan build it",
    "/loop 3 improve it",
    "/commit",
    "/commit apply",
    "/review",
    "/standup 3",
    "/resume",
    "/goal ship the feature",
    "/plan migrate the config",
    "/approve",
    "/reject",
    "/skill review lib.rs",
    "/memory",
    "/mcp",
];

#[test]
fn retired_commands_teach_the_plain_language_ask() {
    for cmd in RETIRED_FORMS {
        let evs = run(cmd);
        let t = text_of(&evs[0]);
        assert!(t.contains("retired"), "{cmd}: {t}");
        // The dedicated hint, not the generic unknown-construct arm: the user
        // must be taught the phrasing, not offered a different slash command.
        assert!(!t.contains("unknown construct"), "{cmd}: {t}");
        assert!(!t.contains("did you mean"), "{cmd}: {t}");
    }
}

#[test]
fn retired_commands_answer_inline() {
    // The hint dials no agent, so a retired command must never occupy a
    // worker slot.
    for cmd in RETIRED_FORMS {
        assert!(is_quick(cmd), "{cmd}");
    }
}

#[test]
fn help_teaches_plain_language_instead_of_retired_commands() {
    let evs = run("/help");
    let t = text_of(&evs[0]);
    for gone in [
        "/fan", "/loop", "/commit", "/review", "/standup", "/resume", "/goal", "/plan", "/approve",
        "/reject", "/skill", "/memory", "/mcp",
    ] {
        // Checked per line-start, not per substring: paths like
        // `.crew/memory.md` legitimately contain "/memory".
        assert!(
            !t.lines().any(|l| l.trim_start().starts_with(gone)),
            "{gone} still advertised: {t}"
        );
    }
    // The replacements: example phrasings the intent router recognizes.
    assert!(t.contains("have every agent take a crack at"), "{t}");
    assert!(t.contains("keep refining"), "{t}");
    assert!(t.contains("keep working until"), "{t}");
    assert!(t.contains("draft a plan"), "{t}");
    assert!(t.contains("approve"), "{t}");
    assert!(t.contains("commit this"), "{t}");
    assert!(t.contains("apply"), "{t}");
    // The idioms that stay: memory capture and the skills drop-in surface.
    assert!(t.contains("#<note>"), "{t}");
    assert!(t.contains("skills"), "{t}");
}

/// The commit hint must carry the gate: a user taught "commit this" must in
/// the same breath be taught that nothing is committed until they confirm.
#[test]
fn the_commit_hint_teaches_the_apply_gate() {
    let evs = run("/commit");
    let t = text_of(&evs[0]);
    assert!(t.contains("commit this"), "{t}");
    assert!(t.contains("apply"), "{t}");
}

/// The plan hints carry the surviving gate the same way: the draft still
/// waits for the user's own verdict, spoken instead of slashed.
#[test]
fn the_plan_hints_teach_the_conversational_gate() {
    for cmd in ["/plan migrate", "/approve", "/reject"] {
        let t_evs = run(cmd);
        let t = text_of(&t_evs[0]);
        assert!(t.contains("approve") || t.contains("reject"), "{cmd}: {t}");
    }
}

/// Retired names must be OUT of the construct list — otherwise the palette
/// drift tests would keep offering them and typo suggestions would resurrect
/// them.
#[test]
fn retired_commands_left_the_construct_list() {
    for gone in [
        "fan", "loop", "commit", "review", "standup", "resume", "goal", "plan", "approve",
        "reject", "skill", "memory", "mcp",
    ] {
        assert!(
            !broker_constructs().contains(&gone),
            "{gone} still a construct"
        );
    }
    assert_eq!(broker_constructs().len(), 9, "{:?}", broker_constructs());
}

/// The nine that remain: session machinery the model cannot or must not
/// decide. Pinned as a list, not just a count, so a rename can't hide.
#[test]
fn the_surviving_constructs_are_the_infrastructure_nine() {
    assert_eq!(
        broker_constructs(),
        &["help", "model", "login", "logout", "doctor", "restore", "reload", "diff", "stop"]
    );
}
