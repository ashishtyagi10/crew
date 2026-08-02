use super::*;
use crate::broker::testenv;

fn info(name: &str, state: AuthState, login: Option<&'static str>, active: bool) -> ProviderInfo {
    ProviderInfo {
        name: name.to_string(),
        state,
        login,
        active,
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
    let note = select(&full_house(), 1);
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
    let note = select(&states, 2);
    assert!(note.contains("dashscope"), "{note}");
    assert_eq!(
        crate::credentials::load().provider.as_deref(),
        Some("dashscope")
    );
    let own = select(&states, 3);
    assert!(own.contains("@opencode"), "{own}");
    assert_eq!(
        crate::credentials::load().provider.as_deref(),
        Some("dashscope"),
        "an own-auth CLI must not move the pin"
    );
    let oob = select(&states, 9);
    assert!(oob.contains("9"), "{oob}");
}
