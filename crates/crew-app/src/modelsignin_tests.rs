use super::*;

fn opt(name: &str, device: bool, signed_in: bool) -> SignInOption {
    SignInOption {
        name: name.into(),
        device,
        signed_in,
        key_present: false,
        login: (!device).then(|| format!("{name} login")),
        install: None,
        logout: None,
    }
}

/// The section leads with its header, carries every broker row as a
/// `/model <name>` submit, marks who serves, dims a signed-out CLI (its
/// desc names the command), and ends with OpenRouter's browser flow.
#[test]
fn rows_submit_model_provider_and_say_who_serves() {
    let opts = vec![
        opt("dashscope", true, false),
        opt("claude-code", false, true),
        opt("codex", false, false),
    ];
    let r = section(&opts, false, Some("claude-code"), "");
    assert!(r[0].header && r[0].label.starts_with("sign in"));
    let labels: Vec<&str> = r[1..].iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, ["dashscope", "claude-code", "codex", "openrouter"]);
    assert_eq!(r[1].fill, "/model dashscope");
    assert!(r[1].submit && !r[1].dim, "a device flow is always pickable");
    assert!(r[1].desc.contains("OAuth"), "{}", r[1].desc);
    assert_eq!(r[2].fill, "/model claude-code");
    assert!(r[2].desc.contains("serving"), "{}", r[2].desc);
    assert!(!r[2].dim);
    assert!(r[3].dim, "a signed-out CLI is dim");
    assert!(r[3].desc.contains("codex login"), "{}", r[3].desc);
    assert_eq!(
        r[4].needs.as_deref(),
        Some(crate::oauth::OPENROUTER_KEY_VAR)
    );
    assert!(r.iter().all(|i| i.label != "sign in" || i.header));
}

/// A signed-in row that is NOT serving says the pick makes it serve —
/// the whole point of the row for a user whose pin is elsewhere.
#[test]
fn a_signed_in_row_not_serving_offers_to_serve() {
    let r = section(
        &[opt("claude-code", false, true)],
        true,
        Some("dashscope"),
        "",
    );
    assert!(r[1].desc.contains("pick to make it serve"), "{}", r[1].desc);
    assert!(r[2].desc.contains("key present"), "{}", r[2].desc);
}

/// The query filters by name, and a section never renders empty — nor at
/// all before a broker has sent its rows.
#[test]
fn the_query_filters_and_the_section_is_never_empty() {
    let opts = vec![
        opt("dashscope", true, false),
        opt("claude-code", false, true),
    ];
    let r = section(&opts, false, None, "cla");
    let labels: Vec<&str> = r.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, ["sign in \u{00b7} who serves", "claude-code"]);
    assert!(section(&opts, false, None, "zzz").is_empty());
    assert!(
        section(&[], false, None, "").is_empty(),
        "no broker rows, no section"
    );
    assert!(signed_in(&opts, "CLAUDE-CODE"));
    assert!(!signed_in(&opts, "dashscope"));
}

/// A sign-in row's fill is a whole construct, and the palette runs it as
/// one — where a model row's slug becomes `/model all <slug>`.
#[test]
fn a_sign_in_pick_runs_model_provider_where_a_model_pick_runs_model_all() {
    use crate::chatpalette::{accept, Kind};
    assert_eq!(
        accept("/model cl", Kind::Model, "/model claude-code"),
        "/model claude-code"
    );
    assert_eq!(
        accept("/model cl", Kind::Model, "claude-sonnet-5"),
        "/model all claude-sonnet-5"
    );
}
