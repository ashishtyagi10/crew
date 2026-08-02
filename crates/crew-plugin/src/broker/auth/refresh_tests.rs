//! The refresh half against the same scripted loopback OAuth server as
//! `device_tests`, plus the pure endpoint normalization. Reauth state is
//! keyed by unique provider names so parallel tests never share a flag.
use super::*;
use std::io::{Read, Write};

/// Serve `script` responses in order; returns the base URL.
fn serve(script: Vec<(u16, String)>) -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for (status, json) in script {
            let Ok((mut sock, _)) = listener.accept() else {
                return;
            };
            let mut buf = [0u8; 4096];
            let _ = sock.read(&mut buf);
            let reply = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\n\
                 content-length: {}\r\nconnection: close\r\n\r\n{json}",
                json.len()
            );
            let _ = sock.write_all(reply.as_bytes());
        }
    });
    base
}

fn ends(base: &str) -> DeviceEndpoints {
    DeviceEndpoints {
        device_url: format!("{base}/device"),
        token_url: format!("{base}/token"),
        client_id: "cid".into(),
        scope: "s".into(),
    }
}

#[test]
fn refresh_success_stores_the_new_grant_and_keeps_omitted_fields() {
    // The refresh reply omits refresh_token and resource_url — the stored
    // grant must keep the old ones rather than losing the ability to refresh.
    let base = serve(vec![(
        200,
        r#"{"access_token":"at-new","expires_in":7200}"#.into(),
    )]);
    let old = StoredToken {
        access: "at-old".into(),
        refresh: Some("rt-old".into()),
        expires_at: 10, // long expired at now=1000
        resource: Some("https://portal/v1".into()),
    };
    let mut calls = Vec::new();
    let got = fresh_with("prov-refresh-ok", &ends(&base), old, 1000, &mut |p, t| {
        calls.push((p.to_string(), t));
    });
    assert_eq!(
        got,
        Some(("at-new".to_string(), Some("https://portal/v1".to_string())))
    );
    assert_eq!(calls.len(), 1);
    let t = calls[0]
        .1
        .as_ref()
        .expect("a refresh success stores a grant");
    assert_eq!(t.refresh.as_deref(), Some("rt-old"));
    assert_eq!(t.expires_at, 1000 + 7200 - tokens::EXPIRY_SKEW_SECS);
}

#[test]
fn a_fresh_token_is_returned_without_any_http() {
    // Endpoints point at a dead port: any request would error the flow.
    let dead = ends("http://127.0.0.1:1");
    let tok = StoredToken {
        access: "at-live".into(),
        refresh: None,
        expires_at: u64::MAX,
        resource: None,
    };
    let got = fresh_with("prov-no-http", &dead, tok, 1000, &mut |_, _| {
        panic!("an unexpired grant must not touch storage")
    });
    assert_eq!(got, Some(("at-live".to_string(), None)));
}

#[test]
fn a_hard_refresh_failure_discards_the_grant_and_arms_one_prompt() {
    let base = serve(vec![
        (400, r#"{"error":"invalid_grant"}"#.into()),
        (400, r#"{"error":"invalid_grant"}"#.into()),
    ]);
    let expired = || StoredToken {
        access: "at-dead".into(),
        refresh: Some("rt-dead".into()),
        expires_at: 10,
        resource: None,
    };
    let mut calls = Vec::new();
    let e = ends(&base);
    let mut sink = |p: &str, t: Option<StoredToken>| calls.push((p.to_string(), t.is_some()));
    assert_eq!(
        fresh_with("prov-hard-fail", &e, expired(), 1000, &mut sink),
        None
    );
    assert_eq!(
        fresh_with("prov-hard-fail", &e, expired(), 1000, &mut sink),
        None
    );
    // Both failures discarded the grant…
    assert_eq!(
        calls,
        [
            ("prov-hard-fail".to_string(), false),
            ("prov-hard-fail".to_string(), false)
        ]
    );
    // …but the prompt fires EXACTLY once until a new sign-in clears it.
    let prompts = (0..3).filter(|_| take_reauth("prov-hard-fail")).count();
    assert_eq!(prompts, 1, "one re-auth prompt, then silence");
    clear_reauth("prov-hard-fail");
    assert!(
        !take_reauth("prov-hard-fail"),
        "cleared state prompts nothing"
    );
}

/// A bare host, a base URL, and already-complete endpoints all normalize to
/// one OpenAI-wire chat endpoint (Qwen returns the bare-host form).
#[test]
fn resource_urls_normalize_to_a_chat_endpoint() {
    let table = [
        (
            "portal.qwen.ai",
            "https://portal.qwen.ai/v1/chat/completions",
        ),
        (
            "https://portal.qwen.ai/v1",
            "https://portal.qwen.ai/v1/chat/completions",
        ),
        (
            "http://127.0.0.1:8080",
            "http://127.0.0.1:8080/v1/chat/completions",
        ),
        (
            "https://x/v1/chat/completions",
            "https://x/v1/chat/completions",
        ),
    ];
    for (given, want) in table {
        assert_eq!(chat_endpoint(given), want, "{given}");
    }
}

/// The stand-in only answers for device-flow providers — an ordinary keyed
/// var can never grow a token out of nowhere.
#[test]
fn key_stand_in_ignores_non_device_vars() {
    assert_eq!(key_stand_in("OPENROUTER_API_KEY"), None);
    assert_eq!(key_stand_in("NOT_A_VAR"), None);
}
