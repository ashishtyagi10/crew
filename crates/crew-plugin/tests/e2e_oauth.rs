//! THE HEADLINE (condition 1 + the rest of 7): a clean HOME, an empty
//! environment — no `*_API_KEY` anywhere, the harness strips them — and a
//! stub OAuth + OpenAI-wire server. `/model` shows the sign-in affordance,
//! picking its number runs the device flow in-pane (code card as a chat
//! message, poll, stored grant), and the SAME session then answers a plain
//! message, runs a fan-out, and drafts a plan — every model call served
//! through the resolution chain by the grant. Afterwards the e2e_secrets
//! sweep runs again over the REAL flow's leavings.
mod common;
use common::oauthstub::{chat_reply, serve, sweep};
use common::{messages, run_broker_paced, seed_specialists, unique_dir};

const SECRET: &str = "e2e-secret-77";

fn send(text: &str) -> String {
    serde_json::json!({"type": "send", "channel": "crew", "text": text}).to_string()
}

/// The scripted provider, self-referencing: the token grant names the stub
/// ITSELF as `resource_url`, so the grant-endpoint wiring (Qwen tokens serve
/// at the host the grant names, not the key-shaped endpoint) is what routes
/// every chat call here. The base URL is only known after binding, hence the
/// holder the handler reads.
fn provider_stub() -> common::oauthstub::StubServer {
    let base_holder = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let for_handler = std::sync::Arc::clone(&base_holder);
    let stub = serve(move |path, body| {
        let me = for_handler.lock().unwrap().clone();
        route(path, body, &me)
    });
    *base_holder.lock().unwrap() = stub.base.clone();
    stub
}

#[test]
fn zero_key_onboarding_signs_in_answers_fans_out_and_plans() {
    let dir = unique_dir("oauth-zero");
    seed_specialists(&dir, &["scout", "sage"]);
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let stub = provider_stub();
    let events = run_broker_paced(
        &dir,
        &[
            ("HOME", home.to_str().unwrap()),
            ("CREW_OAUTH_BASE", &stub.base),
        ],
        &[
            (&send("/model"), 500),
            (&send("/model 1"), 4500),
            (&send("say hello please"), 3000),
            (&send("@scout+sage compare the two approaches"), 3000),
            (&send("draft a plan for the release notes"), 3000),
        ],
    );
    let msgs = messages(&events);
    let all: String = msgs.iter().map(|(s, t)| format!("{s}: {t}\n")).collect();

    // The affordance: dashscope is numbered row 1, offering the sign-in.
    assert!(
        all.contains("1. dashscope \u{2014} signed out \u{00b7} pick this number to sign in"),
        "{all}"
    );
    // The code card streamed as a normal chat message, code + URL visible.
    let card = msgs
        .iter()
        .find(|(_, t)| t.contains("WDJB-MJHT"))
        .map(|(_, t)| t.clone())
        .expect("the code card must stream into the chat");
    assert!(card.contains("https://qwen.example/activate"), "{card}");
    assert!(card.contains("waiting for approval"), "{card}");
    // The poll succeeded and the provider now serves.
    assert!(
        all.contains("\u{2713} signed in \u{2014} dashscope now serves smith work"),
        "{all}"
    );
    // A plain message answered THROUGH the grant-backed provider.
    assert!(all.contains("stub answer: all good"), "{all}");
    // The fan-out reached both specialists.
    let fanned = msgs
        .iter()
        .filter(|(s, t)| {
            (s.starts_with("scout") || s.starts_with("sage")) && t.contains("stub answer")
        })
        .count();
    assert!(
        fanned >= 3,
        "plain reply + two fan legs, got {fanned}: {all}"
    );
    // The plan draft arrived and waits for a verdict.
    assert!(all.contains("plan ready"), "{all}");
    assert!(all.contains("1. write the notes"), "{all}");

    // The stub saw the documented traffic: one device grant, one token
    // redemption, and every model call flavor.
    let seen = stub.seen.lock().unwrap().clone();
    let count = |p: &str, marker: &str| {
        seen.iter()
            .filter(|(path, body)| path == p && body.contains(marker))
            .count()
    };
    assert_eq!(count("/device", ""), 1, "one device-authorization request");
    assert_eq!(count("/token", "device_code"), 1, "one successful poll");
    assert_eq!(
        count("/v1/chat/completions", "You route a user's message"),
        2,
        "two classify calls (plain message + plan phrasing)"
    );
    assert_eq!(
        count("/v1/chat/completions", "You are in plan mode"),
        1,
        "one plan draft call"
    );

    // The grant landed 0600 in the isolated store…
    let tokens = dir.join("tokens.json");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&tokens).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "token store mode");
    }
    // …and the REAL flow leaked nothing: the e2e_secrets sweep, re-run over
    // a genuine sign-in + traffic (condition 6 at full strength).
    let stderr = std::fs::read_to_string(dir.join("broker-stderr.log")).unwrap_or_default();
    let hits = sweep(
        &dir,
        &[("stdout", &all), ("stderr", &stderr)],
        SECRET,
        &["tokens.json"],
    );
    assert_eq!(hits, Vec::<String>::new(), "token material leaked");
}

/// Shared route logic for the self-referencing stub.
fn route(path: &str, body: &str, me: &str) -> (u16, String) {
    match path {
        "/device" => (
            200,
            r#"{"device_code":"dev-e2e","user_code":"WDJB-MJHT","verification_uri":"https://qwen.example/activate","interval":1,"expires_in":900}"#.into(),
        ),
        "/token" if body.contains("grant_type=refresh_token") => (
            200,
            format!(
                r#"{{"access_token":"at-refreshed-{SECRET}","refresh_token":"rt-{SECRET}","expires_in":3600,"resource_url":"{me}"}}"#
            ),
        ),
        "/token" => (
            200,
            format!(
                r#"{{"access_token":"at-{SECRET}","refresh_token":"rt-{SECRET}","expires_in":3600,"resource_url":"{me}"}}"#
            ),
        ),
        "/v1/chat/completions" if body.contains("You are in plan mode") => {
            (200, chat_reply("1. write the notes\n2. check the tags"))
        }
        "/v1/chat/completions" if body.contains("You route a user's message") => {
            // Decide by the task line, not the whole prompt — the grammar
            // itself contains the words "draft a plan".
            let shape = if body.contains("Message: draft a plan") {
                "SHAPE: plan"
            } else {
                "SHAPE: reply"
            };
            (200, chat_reply(shape))
        }
        "/v1/chat/completions" => (200, chat_reply("stub answer: all good\n@done")),
        other => (404, format!(r#"{{"error":"no route {other}"}}"#)),
    }
}

/// The v0.12.0 field report: a key in the environment hid the OAuth path
/// entirely. `/login` must list the numbered sign-in ANYWAY, run the flow,
/// and — because an explicit sign-in outranks a lying-around key — the
/// GRANT must serve the chat afterward: the stub is only reachable through
/// the grant's self-named `resource_url`, so a reply proves the decoy key
/// (which points nowhere near the stub) was not used. `/logout` then
/// removes the grant.
#[test]
fn login_signs_in_over_a_present_key_and_the_grant_serves() {
    let dir = unique_dir("oauth-login-keyed");
    seed_specialists(&dir, &["scout"]);
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let stub = provider_stub();
    let events = run_broker_paced(
        &dir,
        &[
            ("HOME", home.to_str().unwrap()),
            ("CREW_OAUTH_BASE", &stub.base),
            ("DASHSCOPE_API_KEY", "sk-decoy-not-a-real-key"),
        ],
        &[
            (&send("/login"), 500),
            (&send("/login 1"), 4500),
            (&send("say hello please"), 3000),
            (&send("/logout dashscope"), 500),
        ],
    );
    let msgs = messages(&events);
    let all: String = msgs.iter().map(|(s, t)| format!("{s}: {t}\n")).collect();
    // The key did not hide the affordance: dashscope is numbered row 1.
    assert!(
        all.contains(
            "1. dashscope \u{2014} key present \u{b7} /login 1 signs in with OAuth instead"
        ),
        "{all}"
    );
    // The flow ran in-pane and landed.
    assert!(all.contains("WDJB-MJHT"), "code card must stream: {all}");
    assert!(
        all.contains("\u{2713} signed in \u{2014} dashscope now serves smith work"),
        "{all}"
    );
    // The grant serves: the reply came through the stub's chat route, which
    // only the grant's resource_url reaches.
    assert!(all.contains("stub answer: all good"), "{all}");
    let seen = stub.seen.lock().unwrap().clone();
    assert!(
        seen.iter().any(|(p, _)| p == "/v1/chat/completions"),
        "chat must go through the grant endpoint: {seen:?}"
    );
    // /logout removes the grant and hands back to the key.
    assert!(all.contains("dashscope's grant removed"), "{all}");
}
