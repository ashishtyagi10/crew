use super::*;
use crate::broker::testenv;

fn info(name: &str, state: AuthState, login: Option<&'static str>, active: bool) -> ProviderInfo {
    ProviderInfo {
        name: name.to_string(),
        state,
        login,
        active,
        device: false,
    }
}

/// Unwrap the note a pick answers with (panics on a sign-in verdict).
fn note_of(p: Pick) -> String {
    match p {
        Pick::Note(n) => n,
        other => panic!("wanted a note, got {other:?}"),
    }
}

/// A machine with one signed-in subscription, one signed-out CLI, one key,
/// one keyless provider and one own-auth CLI — every listing case at once.
fn full_house() -> Vec<ProviderInfo> {
    vec![
        info(
            "claude-code",
            AuthState::SignedIn,
            Some("claude auth login"),
            false,
        ),
        info("codex", AuthState::SignedOut, Some("codex login"), false),
        info("dashscope", AuthState::KeyPresent, None, true),
        info("openai", AuthState::NoKey, None, false),
        info("opencode", AuthState::Installed, None, false),
    ]
}

/// The grouped listing: three groups in order, numbered continuously, the
/// signed-out provider grayed (unnumbered) with its exact sign-in command,
/// keyless providers absent, and the active provider marked.
#[test]
fn groups_render_numbered_with_signin_hints() {
    let states = full_house();
    let text = groups_text(&states);
    let subs = text
        .find("your subscriptions")
        .expect("subscriptions group");
    let keys = text.find("your keys").expect("keys group");
    let clis = text.find("installed CLIs").expect("installed group");
    assert!(subs < keys && keys < clis, "group order: {text}");
    assert!(text.contains("1. claude-code"), "{text}");
    assert!(text.contains("2. dashscope"), "{text}");
    assert!(text.contains("3. opencode"), "{text}");
    // The signed-out provider is grayed, unnumbered, with the exact command.
    assert!(text.contains("\u{25cb} codex"), "{text}");
    assert!(text.contains("codex login"), "{text}");
    assert!(
        !text.contains(". codex"),
        "codex must not be numbered: {text}"
    );
    // No key, no line — /doctor is where absences are explained.
    assert!(!text.contains("openai"), "{text}");
    assert!(
        text.contains("active"),
        "the active provider is marked: {text}"
    );
    // The numbers agree with the selectable list, by construction.
    let names: Vec<&str> = selectable(&states)
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(names, ["claude-code", "dashscope", "opencode"]);
}

/// A group with nothing in it does not render its header.
#[test]
fn empty_groups_do_not_render() {
    let text = groups_text(&[info("openai", AuthState::NoKey, None, false)]);
    for header in ["your subscriptions", "your keys", "installed CLIs"] {
        assert!(!text.contains(header), "{header} must not render: {text}");
    }
    assert!(
        text.contains(super::super::discover::no_provider_advice()),
        "with nothing usable, say how to get something: {text}"
    );
}

/// `/model <n>` pins the delegated provider through the credential store —
/// the same pin every model choice writes — so it persists across restarts.
#[test]
fn selecting_a_subscription_pins_the_provider() {
    let _g = testenv::no_provider();
    let note = note_of(select(&full_house(), 1));
    assert!(note.contains("claude-code"), "{note}");
    assert_eq!(
        crate::credentials::load().provider.as_deref(),
        Some("claude-code"),
        "the pin must be stored"
    );
}

/// Selecting a keyed provider pins it too; an own-auth CLI is explained,
/// not pinned; out of range says so.
#[test]
fn selection_edges_pin_explain_or_refuse() {
    let _g = testenv::no_provider();
    let states = full_house();
    let note = note_of(select(&states, 2));
    assert!(note.contains("dashscope"), "{note}");
    assert_eq!(
        crate::credentials::load().provider.as_deref(),
        Some("dashscope")
    );
    let own = note_of(select(&states, 3));
    assert!(own.contains("@opencode"), "{own}");
    assert_eq!(
        crate::credentials::load().provider.as_deref(),
        Some("dashscope"),
        "an own-auth CLI must not move the pin"
    );
    let oob = note_of(select(&states, 9));
    assert!(oob.contains("9"), "{oob}");
}

/// A signed-out DEVICE-FLOW provider is a numbered row (its sign-in runs in
/// the pane), it shifts the later numbers, and picking it answers `SignIn` —
/// not a pin, and never a note.
#[test]
fn a_device_flow_sign_in_is_numbered_and_selected() {
    let mut states = full_house();
    let mut d = info("qwen-dev", AuthState::SignedOut, None, false);
    d.device = true;
    states.insert(1, d);
    let text = groups_text(&states);
    assert!(
        text.contains("2. qwen-dev \u{2014} signed out \u{b7} pick this number to sign in"),
        "{text}"
    );
    assert!(text.contains("3. dashscope"), "later numbers shift: {text}");
    assert_eq!(select(&states, 2), Pick::SignIn("qwen-dev".into()));
    let names: Vec<&str> = selectable(&states)
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(names, ["claude-code", "qwen-dev", "dashscope", "opencode"]);
}

/// A device-flow provider serving from a KEY still points at the sign-in:
/// `/login <name>` rides the "key present" detail, so holding a key can
/// never hide the OAuth path (the v0.12.0 report). A plain keyed provider
/// stays a bare "key present".
#[test]
fn a_keyed_device_provider_advertises_login() {
    let mut states = full_house();
    states[2].device = true; // dashscope: KeyPresent + device flow
    let text = groups_text(&states);
    assert!(
        text.contains(
            "dashscope \u{2014} key present \u{b7} /login dashscope signs in with OAuth instead"
        ),
        "{text}"
    );
    states[2].device = false;
    let plain = groups_text(&states);
    assert!(!plain.contains("/login"), "{plain}");
}
