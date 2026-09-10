use super::*;

fn opt(name: &str, device: bool, signed_in: bool, key_present: bool) -> SignInOption {
    SignInOption {
        name: name.into(),
        device,
        signed_in,
        key_present,
        login: (!device).then(|| format!("{name} login")),
        install: None,
        logout: None,
    }
}

fn labels(items: &[MenuItem]) -> Vec<String> {
    items.iter().map(|i| i.label.clone()).collect()
}

/// The sign-in wording the `/model` picker's rows borrow: an uninstalled
/// CLI names its install, a keyed device flow says the sign-in outranks the
/// key, a signed-out CLI names its command, a signed-in device flow still
/// offers itself (a re-sign-in is a legitimate pick).
#[test]
fn state_names_installs_keys_commands_and_re_signins() {
    let mut ant = opt("anthropic", false, false, false);
    ant.install = Some("brew install ant".into());
    assert_eq!(state(&ant), "not installed \u{00b7} `brew install ant`");
    assert_eq!(
        state(&opt("dashscope", true, false, true)),
        "key present \u{00b7} sign in with OAuth instead"
    );
    assert_eq!(
        state(&opt("codex", false, false, false)),
        "signs in through its CLI \u{00b7} run `codex login`"
    );
    assert_eq!(
        state(&opt("codex", false, true, false)),
        "\u{2713} signed in through its CLI"
    );
    assert_eq!(
        state(&opt("dashscope", true, true, false)),
        "\u{2713} signed in \u{00b7} pick to sign in again"
    );
    assert_eq!(
        state(&opt("qwen", true, false, false)),
        "sign in with OAuth"
    );
}

/// `open` lands the selection on the first choice, never the header, and
/// the accepted row becomes the broker construct.
#[test]
fn open_selects_the_first_choice_and_accept_forms_the_construct() {
    let mut p = None;
    open(&mut p, &[opt("dashscope", true, true, false)]);
    let p = p.expect("opened");
    assert_eq!(p.kind, Kind::SignOut);
    assert_eq!(p.items[p.sel].label, "dashscope");
    assert_eq!(
        crate::chatpalette::accept("", Kind::SignOut, "dashscope"),
        "/logout dashscope"
    );
    assert_eq!(LEGEND, "sign out");
}

/// `/logout`'s rows: crew's grants first (a pick removes one, and a held
/// key is named as what serves next), CLI-owned sign-ins dimmed with their
/// own sign-out command — or the fact that the CLI owns it.
#[test]
fn logout_rows_lead_with_grants_and_dim_the_clis() {
    let mut ant = opt("anthropic", false, true, false);
    ant.logout = Some("ant auth logout".into());
    let o = [
        opt("dashscope", true, true, true),
        ant,
        opt("codex", false, true, false),
    ];
    let items = items_out(&o);
    assert_eq!(
        labels(&items),
        [
            "stored grants \u{00b7} pick to remove",
            "dashscope",
            "vendor CLIs \u{00b7} sign out at the terminal",
            "anthropic",
            "codex",
        ]
    );
    assert!(items[1].submit && !items[1].dim);
    assert_eq!(
        items[1].desc,
        "\u{2713} signed in \u{00b7} the key serves once removed"
    );
    assert!(items[3].dim && items[3].desc.ends_with("run `ant auth logout`"));
    assert!(items[4].dim && items[4].desc.ends_with("it owns the sign-out"));
    let mut p = None;
    open(&mut p, &o);
    assert_eq!(p.as_ref().unwrap().kind, Kind::SignOut);
    // No CLI rows: no CLI header either.
    assert_eq!(items_out(&o[..1]).len(), 2);
}

/// The `/logout` picker in a frame.
///
/// `#[ignore]`d (needs a GPU adapter, writes PNGs):
/// `cargo test -p crew-app --bin crew pick_shot_logout -- --ignored --nocapture`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn pick_shot_logout() {
    let _g = crate::app::theme_test_guard();
    let mut ant = opt("anthropic", false, true, false);
    ant.logout = Some("ant auth logout".into());
    let o = [
        opt("dashscope", true, true, true),
        ant,
        opt("codex", false, true, false),
    ];
    let items = items_out(&o);
    let Some(rows) = crate::pickshot_tests::menu_shot("logout", LEGEND, &items, 1, 760) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    assert!(rows.iter().any(|r| r.contains("stored grants")), "{rows:?}");
}
