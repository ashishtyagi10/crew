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

/// Runnable sign-ins first, the browser flow among them, vendor CLIs in a
/// section of their own — and every runnable row submits on Enter.
#[test]
fn runnable_signins_lead_and_clis_follow_in_their_own_section() {
    let o = [
        opt("codex", false, true, false),
        opt("dashscope", true, false, true),
        opt("claude-code", false, false, false),
    ];
    let items = items(&o, false);
    assert_eq!(
        labels(&items),
        [
            "OAuth \u{00b7} runs right here",
            "dashscope",
            "openrouter",
            "vendor CLIs \u{00b7} sign in at the terminal",
            "codex",
            "claude-code",
        ]
    );
    assert!(items[0].header && items[3].header);
    assert!(items[1].submit && items[1].fill == "dashscope");
    assert_eq!(
        items[1].desc,
        "key present \u{00b7} sign in with OAuth instead"
    );
    // A signed-out CLI row is dimmed and says what to run; a signed-in one is not.
    assert!(items[5].dim && items[5].desc.contains("run `claude-code login`"));
    assert!(!items[4].dim && items[4].desc.contains("signed in"));
}

/// The browser flow is the app's own: its row is the model picker's `needs`
/// row, so picking it opens the browser and the paste prompt, never a submit.
#[test]
fn openrouter_is_a_needs_row_that_reports_a_held_key() {
    let or = |keyed: bool| {
        items(&[], keyed)
            .into_iter()
            .find(|i| i.label == "openrouter")
            .unwrap()
    };
    let row = or(false);
    assert_eq!(row.needs.as_deref(), Some(crate::oauth::OPENROUTER_KEY_VAR));
    assert!(!row.submit);
    assert_eq!(row.desc, "sign in in the browser");
    assert!(or(true).desc.starts_with("\u{2713} key present"));
}

/// An uninstalled CLI names its install; a signed-in device flow still
/// offers itself (a re-sign-in is a legitimate pick).
#[test]
fn rows_name_installs_and_re_signins() {
    let mut ant = opt("anthropic", false, false, false);
    ant.install = Some("brew install ant".into());
    let items = items(&[ant, opt("dashscope", true, true, false)], false);
    let by = |n: &str| items.iter().find(|i| i.label == n).unwrap().desc.clone();
    assert_eq!(by("anthropic"), "not installed \u{00b7} `brew install ant`");
    assert_eq!(
        by("dashscope"),
        "\u{2713} signed in \u{00b7} pick to sign in again"
    );
}

/// `open` lands the selection on the first choice, never the header, and
/// the accepted row becomes the broker construct.
#[test]
fn open_selects_the_first_choice_and_accept_forms_the_construct() {
    let mut p = None;
    open(&mut p, Auth::In, &[opt("dashscope", true, false, false)]);
    let p = p.expect("opened");
    assert_eq!(p.kind, Kind::Auth(Auth::In));
    assert_eq!(p.items[p.sel].label, "dashscope");
    let accept = crate::chatpalette::accept;
    assert_eq!(
        accept("", Kind::Auth(Auth::In), "dashscope"),
        "/login dashscope"
    );
    assert_eq!(
        accept("", Kind::Auth(Auth::Out), "dashscope"),
        "/logout dashscope"
    );
    assert_eq!(
        (Auth::In.legend(), Auth::Out.legend()),
        ("sign in", "sign out")
    );
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
    open(&mut p, Auth::Out, &o);
    assert_eq!(p.as_ref().unwrap().kind, Kind::Auth(Auth::Out));
    // No CLI rows: no CLI header either.
    assert_eq!(items_out(&o[..1]).len(), 2);
}

/// The picker in a frame: the sign-in header, the runnable rows, the browser
/// row, and the dimmed vendor CLIs under their own header.
///
/// `#[ignore]`d (needs a GPU adapter, writes PNGs):
/// `cargo test -p crew-app --bin crew pick_shot_login -- --ignored --nocapture`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn pick_shot_login() {
    let _g = crate::app::theme_test_guard();
    let mut ant = opt("anthropic", false, false, false);
    ant.install = Some("brew install ant".into());
    let o = [
        opt("dashscope", true, false, true),
        opt("claude-code", false, true, false),
        opt("codex", false, false, false),
        ant,
    ];
    let items = items(&o, false);
    let Some(rows) = crate::pickshot_tests::menu_shot("login", "sign in", &items, 1, 760) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    assert!(rows.iter().any(|r| r.contains("dashscope")), "{rows:?}");
    assert!(rows.iter().any(|r| r.contains("vendor CLIs")), "{rows:?}");
}

/// The `/logout` picker in a frame.
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
    let Some(rows) = crate::pickshot_tests::menu_shot("logout", "sign out", &items, 1, 760) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    assert!(rows.iter().any(|r| r.contains("stored grants")), "{rows:?}");
}
