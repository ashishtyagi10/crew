use super::*;

fn device(name: &str, signed_in: bool, key_present: bool) -> LoginRow {
    LoginRow {
        name: name.to_string(),
        cli_login: None,
        device: true,
        signed_in,
        key_present,
    }
}

fn delegated(name: &str, login: &'static str, signed_in: bool) -> LoginRow {
    LoginRow {
        name: name.to_string(),
        cli_login: Some(login),
        device: false,
        signed_in,
        key_present: false,
    }
}

/// THE bug this file exists for: a device provider with an API key present
/// must still render a NUMBERED sign-in row — the key used to hide the
/// OAuth path entirely.
#[test]
fn a_key_present_device_provider_still_offers_the_numbered_signin() {
    let rows = vec![device("dashscope", false, true)];
    let text = listing(&rows);
    assert!(text.contains("1. dashscope"), "{text}");
    assert!(text.contains("key present"), "{text}");
    assert!(
        text.contains("/login 1 signs in with OAuth instead"),
        "{text}"
    );
}

#[test]
fn listing_marks_grant_and_delegated_states() {
    let rows = vec![
        device("dashscope", true, true),
        delegated("claude-code", "claude auth login", false),
        delegated("codex", "codex login", true),
    ];
    let text = listing(&rows);
    assert!(
        text.contains("1. dashscope \u{2014} \u{2713} signed in"),
        "{text}"
    );
    assert!(text.contains("/logout"), "{text}");
    // Delegated rows are grayed (unnumbered) with the exact command.
    assert!(text.contains("\u{25cb} claude-code"), "{text}");
    assert!(text.contains("run `claude auth login`"), "{text}");
    assert!(!text.contains(". claude-code"), "{text}");
    assert!(
        text.contains("\u{25cb} codex \u{2014} \u{2713} signed in (vendor CLI)"),
        "{text}"
    );
}

#[test]
fn empty_rows_fall_back_to_the_shared_advice() {
    let text = listing(&[]);
    assert!(
        text.contains(super::super::discover::no_provider_advice()),
        "{text}"
    );
}

/// The numbers `pick` resolves are exactly the ones `listing` prints:
/// device rows only, in order — a delegated row between them must not shift
/// the numbering.
#[test]
fn pick_by_number_indexes_only_the_device_rows() {
    let rows = vec![
        device("dashscope", false, true),
        delegated("claude-code", "claude auth login", false),
        device("qwen-intl", false, false),
    ];
    assert_eq!(pick(&rows, "1"), LoginPick::Device("dashscope".into()));
    assert_eq!(pick(&rows, "2"), LoginPick::Device("qwen-intl".into()));
    let LoginPick::Note(n) = pick(&rows, "3") else {
        panic!("out of range must be a note");
    };
    assert!(n.contains("1..=2"), "{n}");
}

#[test]
fn pick_by_name_is_case_insensitive_and_routes_delegated_to_their_cli() {
    let rows = vec![
        device("dashscope", false, false),
        delegated("claude-code", "claude auth login", false),
    ];
    assert_eq!(
        pick(&rows, "DashScope"),
        LoginPick::Device("dashscope".into())
    );
    let LoginPick::Note(n) = pick(&rows, "claude-code") else {
        panic!("delegated pick must be a note");
    };
    assert!(n.contains("claude auth login"), "{n}");
}

/// A registry provider with no sign-in flow (openrouter) gets pointed at
/// `/model`; a name the registry has never heard of gets called unknown —
/// two different mistakes, two different answers.
#[test]
fn pick_distinguishes_keyed_only_providers_from_unknown_names() {
    let rows = vec![device("dashscope", false, false)];
    let LoginPick::Note(keyed) = pick(&rows, "openrouter") else {
        panic!("keyed-only pick must be a note");
    };
    assert!(keyed.contains("no sign-in flow"), "{keyed}");
    assert!(keyed.contains("/model"), "{keyed}");
    let LoginPick::Note(unknown) = pick(&rows, "nonesuch") else {
        panic!("unknown pick must be a note");
    };
    assert!(unknown.contains("unknown provider"), "{unknown}");
}
