//! Condition 5 through the real broker binary: an EXPIRED stored grant
//! refreshes transparently before the model call (exactly one refresh
//! round-trip, no prompt, the answer just arrives), and a HARD refresh
//! failure surfaces exactly one re-auth line across any number of later
//! tasks — the grant is discarded, so not even the HTTP repeats.
mod common;
use common::oauthstub::{chat_reply, serve};
use common::{messages, run_broker_paced, seed_specialists, unique_dir};

fn send(text: &str) -> String {
    serde_json::json!({"type": "send", "channel": "crew", "text": text}).to_string()
}

/// An expired grant for dashscope whose chat host is the stub itself.
fn seed_expired_grant(dir: &std::path::Path, stub_base: &str) {
    std::fs::write(
        dir.join("tokens.json"),
        format!(
            r#"{{"dashscope":{{"access":"at-stale","refresh":"rt-refresh-me","expires_at":1,"resource":"{stub_base}"}}}}"#
        ),
    )
    .unwrap();
}

#[test]
fn an_expired_grant_refreshes_transparently_before_the_call() {
    let dir = unique_dir("refresh-ok");
    seed_specialists(&dir, &["scout"]);
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let base_holder = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let for_handler = std::sync::Arc::clone(&base_holder);
    let stub = serve(move |path, body| {
        let me = for_handler.lock().unwrap().clone();
        match path {
            "/token" if body.contains("grant_type=refresh_token") => (
                200,
                format!(
                    r#"{{"access_token":"at-fresh-1","refresh_token":"rt-2","expires_in":3600,"resource_url":"{me}"}}"#
                ),
            ),
            "/v1/chat/completions" if body.contains("You route a user's message") => {
                (200, chat_reply("SHAPE: reply"))
            }
            "/v1/chat/completions" => (200, chat_reply("stub answer: refreshed\n@done")),
            other => (404, format!(r#"{{"error":"no route {other}"}}"#)),
        }
    });
    *base_holder.lock().unwrap() = stub.base.clone();
    seed_expired_grant(&dir, &stub.base);
    let events = run_broker_paced(
        &dir,
        &[
            ("HOME", home.to_str().unwrap()),
            ("CREW_OAUTH_BASE", &stub.base),
        ],
        &[(&send("say hello please"), 3000)],
    );
    let msgs = messages(&events);
    let all: String = msgs.iter().map(|(s, t)| format!("{s}: {t}\n")).collect();
    assert!(all.contains("stub answer: refreshed"), "{all}");
    // Invisible until it can't be: no re-auth prompt anywhere…
    assert_eq!(
        msgs.iter()
            .filter(|(_, t)| t.contains("sign-in expired"))
            .count(),
        0,
        "{all}"
    );
    // …and EXACTLY one refresh round-trip served every model call.
    let seen = stub.seen.lock().unwrap().clone();
    let refreshes = seen
        .iter()
        .filter(|(p, b)| p == "/token" && b.contains("grant_type=refresh_token"))
        .count();
    assert_eq!(refreshes, 1, "one transparent refresh: {seen:?}");
    // The refreshed grant replaced the stale one in the store.
    let store = std::fs::read_to_string(dir.join("tokens.json")).unwrap();
    assert!(store.contains("at-fresh-1"), "{store}");
    assert!(!store.contains("at-stale"), "{store}");
}

#[test]
fn a_hard_refresh_failure_prompts_exactly_once_then_stays_silent() {
    let dir = unique_dir("refresh-hard");
    seed_specialists(&dir, &["scout"]);
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let stub = serve(|path, _body| match path {
        "/token" => (400, r#"{"error":"invalid_grant"}"#.into()),
        other => (404, format!(r#"{{"error":"no route {other}"}}"#)),
    });
    seed_expired_grant(&dir, &stub.base);
    let events = run_broker_paced(
        &dir,
        &[
            ("HOME", home.to_str().unwrap()),
            ("CREW_OAUTH_BASE", &stub.base),
        ],
        &[
            (&send("say hello please"), 2500),
            (&send("and again please"), 2500),
        ],
    );
    let msgs = messages(&events);
    let all: String = msgs.iter().map(|(s, t)| format!("{s}: {t}\n")).collect();
    // ONE re-auth line across BOTH tasks — a prompt per task is a nag loop.
    assert_eq!(
        msgs.iter()
            .filter(|(_, t)| t.contains("dashscope sign-in expired"))
            .count(),
        1,
        "{all}"
    );
    assert!(
        all.contains("open /model and pick it to sign in again"),
        "{all}"
    );
    // The dead grant was discarded on the first failure: exactly one refresh
    // attempt ever reached the server.
    let seen = stub.seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 1, "one refresh attempt, then silence: {seen:?}");
    assert!(
        !dir.join("tokens.json")
            .exists()
            .then(|| std::fs::read_to_string(dir.join("tokens.json")).unwrap())
            .is_some_and(|s| s.contains("dashscope")),
        "the dead grant must be discarded"
    );
}
