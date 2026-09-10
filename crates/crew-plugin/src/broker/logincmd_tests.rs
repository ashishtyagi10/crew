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
/// must still be a pickable sign-in row — the key used to hide the OAuth
/// path entirely. Picking it runs the device flow, key or no key.
#[test]
fn a_key_present_device_provider_still_offers_the_signin() {
    let rows = vec![device("dashscope", false, true)];
    let o = options(&rows);
    assert!(o[0].device && o[0].key_present && !o[0].signed_in, "{o:?}");
    assert_eq!(
        pick(&rows, "dashscope"),
        LoginPick::Device("dashscope".into())
    );
}

/// Numbers are `/model <n>`'s and resolve against the provider listing in
/// `modelpick`, never here: a digit is an unknown provider name, and the
/// note points at `/model`.
#[test]
fn pick_takes_names_only_and_points_numbers_at_model() {
    let rows = vec![device("dashscope", false, true)];
    let LoginPick::Note(n) = pick(&rows, "1") else {
        panic!("a number is not a name");
    };
    assert!(
        n.contains("unknown provider") && n.contains("/model"),
        "{n}"
    );
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
    assert!(
        n.contains("/model claude-code again here") && !n.contains("/login"),
        "the way back is the construct that exists: {n}"
    );
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
/// sign-in: absent, its row carries the install; picking it by name never
/// runs a device flow — the CLI owns the login — and says
/// install-then-login when the CLI is missing, sign-in-then-pick when it
/// is there.
#[test]
fn a_minting_cli_row_names_install_login_and_precedence() {
    let brew = "brew install anthropics/tap/ant";
    let rows = vec![
        device("dashscope", false, false),
        minting("anthropic", false, false, Some(brew)),
    ];
    let o = options(&rows);
    assert_eq!(o[1].install.as_deref(), Some(brew), "{o:?}");
    assert_eq!(o[1].login.as_deref(), Some("ant auth login"), "{o:?}");
    let note = match pick(&rows, "anthropic") {
        LoginPick::Note(n) => n,
        other => panic!("{other:?}"),
    };
    assert!(
        note.contains("can't find on its PATH")
            && note.contains("then `ant auth login`")
            && note.contains("/model anthropic again here"),
        "{note}"
    );
    assert!(
        !note.contains("next pane"),
        "a login is noticed in THIS pane now: {note}"
    );
    assert_eq!(
        pick(&[minting("anthropic", true, true, None)], "anthropic"),
        LoginPick::Serve("anthropic".into()),
        "a signed-in pick makes it serve, never a second login"
    );
}

/// The picker's rows carry every state the table prints, device flows first
/// (a pick can run those), CLI-owned sign-ins after with their command —
/// and nothing that is a credential.
#[test]
fn options_carry_the_states_device_flows_first() {
    let rows = vec![
        delegated("claude-code", "claude auth login", false),
        device("dashscope", false, true),
        minting("anthropic", false, false, Some("brew install ant")),
    ];
    let o = options(&rows);
    let names: Vec<&str> = o.iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, ["dashscope", "claude-code", "anthropic"]);
    assert!(o[0].device && o[0].key_present && !o[0].signed_in);
    assert_eq!(o[1].login.as_deref(), Some("claude auth login"));
    assert_eq!(o[1].install, None);
    assert_eq!(o[2].install.as_deref(), Some("brew install ant"));
    assert_eq!(o[2].login.as_deref(), Some("ant auth login"));
    assert!(options(&[]).is_empty());
}

/// `/logout`'s rows are the signed-in ones only, grants first, and a
/// minting CLI's row carries its own sign-out command (crew runs nothing
/// against a vendor's store).
#[test]
fn signed_in_rows_carry_the_cli_signout() {
    let rows = vec![
        minting("anthropic", true, false, None),
        device("dashscope", true, false),
        device("qwen", false, false),
        delegated("codex", "codex login", true),
    ];
    let o = signed_in(&rows);
    let names: Vec<&str> = o.iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, ["dashscope", "anthropic", "codex"]);
    assert_eq!(o[1].logout.as_deref(), Some("ant auth logout"));
    assert_eq!(o[2].logout, None);
    assert_eq!(o[0].logout, None);
}

/// Picking a CLI row that is already signed in makes it SERVE (the pin
/// moves there — the last choice wins), never a second "run `ant auth
/// login`" — the user just did that, and picked the row to use it.
#[test]
fn picking_a_signed_in_cli_row_makes_it_serve() {
    let rows = vec![minting("anthropic", true, false, None)];
    assert_eq!(
        pick(&rows, "anthropic"),
        LoginPick::Serve("anthropic".into())
    );
    let rows = vec![delegated("claude-code", "claude auth login", true)];
    assert_eq!(
        pick(&rows, "claude-code"),
        LoginPick::Serve("claude-code".into())
    );
    // Signed out and installed: the login command, and how crew notices.
    let rows = vec![delegated("codex", "codex login", false)];
    let note = match pick(&rows, "codex") {
        LoginPick::Note(n) => n,
        other => panic!("{other:?}"),
    };
    assert!(
        note.contains("run `codex login`") && note.contains("/model codex again here"),
        "{note}"
    );
}
