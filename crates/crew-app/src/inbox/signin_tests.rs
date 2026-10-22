use std::io::{Read, Write};
use std::net::TcpListener;

use super::super::grant::{Client, Grant};
use crate::oauth::{parse_request, Callback};

fn client() -> Client {
    Client {
        id: "123-abc.apps.googleusercontent.com".into(),
        secret: "GOCSPX-xyz".into(),
    }
}

/// A one-shot HTTP server: answers `status` with `json` and hands back the
/// request body it was sent.
fn stub(status: u16, json: &'static str) -> (String, std::thread::JoinHandle<String>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/token", l.local_addr().unwrap());
    let h = std::thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap();
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        let body = loop {
            let n = s.read(&mut chunk).unwrap();
            buf.extend_from_slice(&chunk[..n]);
            let text = String::from_utf8_lossy(&buf).to_string();
            if let Some((head, body)) = text.split_once("\r\n\r\n") {
                let len = head
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if body.len() >= len {
                    break body.to_string();
                }
            }
        };
        let reply = format!(
            "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{json}",
            json.len()
        );
        s.write_all(reply.as_bytes()).unwrap();
        body
    });
    (url, h)
}

#[test]
fn the_consent_url_asks_to_read_and_file_mail_with_a_refresh_token_and_pkce() {
    let u = super::authorize_url(
        &client().id,
        "http://127.0.0.1:5555",
        "CHAL",
        "STATE",
        false,
    );
    assert!(u.starts_with("https://accounts.google.com/o/oauth2/v2/auth?client_id=123-abc.apps.googleusercontent.com&"), "{u}");
    for part in [
        "redirect_uri=http%3A%2F%2F127.0.0.1%3A5555",
        "response_type=code",
        "scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fgmail.modify",
        "code_challenge=CHAL",
        "code_challenge_method=S256",
        "state=STATE",
        "access_type=offline",
        "prompt=consent",
    ] {
        assert!(u.contains(part), "{part} missing from {u}");
    }
}

#[test]
fn googles_callback_is_ours_by_its_state() {
    let ours = "GET /?state=abc&code=4%2F0AbC&scope=https%3A%2F%2Fx HTTP/1.1";
    assert!(parse_request(ours, "abc") == Callback::Code("4%2F0AbC".into()));
    assert!(parse_request("GET /?state=zzz&code=4%2F0A HTTP/1.1", "abc") == Callback::Ignore);
    assert!(parse_request("GET /?code=4%2F0A HTTP/1.1", "abc") == Callback::Ignore);
    assert!(
        parse_request("GET /?state=abc&error=access_denied HTTP/1.1", "abc")
            == Callback::Denied("access_denied".into())
    );
    assert_eq!(super::super::query::dec("4%2F0AbC"), "4/0AbC");
}

#[test]
fn a_code_is_redeemed_with_its_verifier_for_a_refresh_token() {
    let (url, sent) = stub(
        200,
        r#"{"access_token":"ya29.a","expires_in":3599,"refresh_token":"1//r","scope":"x","token_type":"Bearer"}"#,
    );
    let g = super::exchange(
        &url,
        &client(),
        "4/0AbC",
        "VERIFIER",
        "http://127.0.0.1:5555",
    )
    .unwrap();
    assert!(
        g == Grant {
            client: client(),
            refresh_token: "1//r".into(),
            email: String::new(),
            // What Google said it granted, kept as it said it.
            scope: "x".into(),
        }
    );
    let body = sent.join().unwrap();
    for part in [
        "grant_type=authorization_code",
        "code=4%2F0AbC",
        "code_verifier=VERIFIER",
        "client_secret=GOCSPX-xyz",
        "redirect_uri=http%3A%2F%2F127.0.0.1%3A5555",
    ] {
        assert!(body.contains(part), "{part} missing from {body}");
    }
}

#[test]
fn an_ended_grant_says_googles_reason_and_nothing_secret() {
    let (url, sent) = stub(
        400,
        r#"{"error":"invalid_grant","error_description":"Token has been expired or revoked."}"#,
    );
    let g = Grant {
        client: client(),
        refresh_token: "1//secret".into(),
        email: String::new(),
        scope: String::new(),
    };
    let err = super::refresh(&url, &g).unwrap_err();
    assert_eq!(err, "invalid_grant: Token has been expired or revoked.");
    assert!(sent.join().unwrap().contains("grant_type=refresh_token"));
}

#[test]
fn a_refresh_gives_the_access_token_and_its_life() {
    let (url, _) = stub(
        200,
        r#"{"access_token":"ya29.b","expires_in":3599,"token_type":"Bearer"}"#,
    );
    let g = Grant {
        client: client(),
        refresh_token: "1//r".into(),
        email: String::new(),
        scope: String::new(),
    };
    assert_eq!(
        super::refresh(&url, &g).unwrap(),
        ("ya29.b".to_string(), 3599)
    );
}

#[test]
fn query_values_encode_and_decode_back() {
    let raw = "a b/c?d=é&~";
    assert_eq!(super::super::query::enc(raw), "a%20b%2Fc%3Fd%3D%C3%A9%26~");
    assert_eq!(
        super::super::query::dec(&super::super::query::enc(raw)),
        raw
    );
    assert_eq!(super::super::query::dec("100%"), "100%", "a stray % stays");
}
