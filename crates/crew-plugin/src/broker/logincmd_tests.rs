use super::*;

fn device(name: &str, signed_in: bool, key_present: bool) -> LoginRow {
    LoginRow {
        name: name.to_string(),
        cli_login: None,
        device: true,
        signed_in,
        key_present,
        install: None,
    }
}

fn delegated(name: &str, login: &'static str, signed_in: bool) -> LoginRow {
    LoginRow {
        name: name.to_string(),
        cli_login: Some(login),
        device: false,
        signed_in,
        key_present: false,
        install: None,
    }
}

fn minting(
    name: &str,
    signed_in: bool,
    key_present: bool,
    install: Option<&'static str>,
) -> LoginRow {
    LoginRow {
        name: name.to_string(),
        cli_login: Some("ant auth login"),
        device: false,
        signed_in,
        key_present,
        install,
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

/// A minting CLI's row is the front door to the sanctioned Anthropic
/// sign-in: absent, it says how to install; signed out beside a key, it
/// says the sign-in outranks the key; signed in, it reads like any vendor
/// CLI. Picking it by name never runs a device flow — the CLI owns the
/// login — and says install-then-login when the CLI is missing.
#[test]
fn a_minting_cli_row_names_install_login_and_precedence() {
    let brew = "brew install anthropics/tap/ant";
    let rows = vec![
        device("dashscope", false, false),
        minting("anthropic", false, false, Some(brew)),
    ];
    let text = listing(&rows);
    assert!(text.contains(" 1. dashscope"), "{text}");
    assert!(
        text.contains("\u{25cb} anthropic \u{2014} not installed \u{00b7} `brew install anthropics/tap/ant`, then `ant auth login`"),
        "{text}"
    );
    let note = match pick(&rows, "anthropic") {
        LoginPick::Note(n) => n,
        other => panic!("{other:?}"),
    };
    assert!(
        note.contains("not installed") && note.contains("then `ant auth login`"),
        "{note}"
    );
    assert_eq!(
        pick(&rows, "2"),
        LoginPick::Note("no sign-in #2 \u{2014} the listing numbers 1..=1".into())
    );

    let text = listing(&[minting("anthropic", false, true, None)]);
    assert!(
        text.contains("key present \u{00b7} `ant auth login` signs in with OAuth instead"),
        "{text}"
    );
    let text = listing(&[minting("anthropic", true, true, None)]);
    assert!(
        text.contains("anthropic \u{2014} \u{2713} signed in (vendor CLI)"),
        "{text}"
    );
    match pick(&[minting("anthropic", true, true, None)], "anthropic") {
        LoginPick::Note(n) => assert!(n.contains("run `ant auth login`"), "{n}"),
        other => panic!("{other:?}"),
    }
}
