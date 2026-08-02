//! Against a scripted local OAuth server: a real `TcpListener` speaking just
//! enough HTTP/1.1 for reqwest, one scripted `(status, json)` response per
//! request, every request body captured for assertions. No network beyond
//! loopback, no real endpoint, no key.
use super::*;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

/// Serve `script` responses in order on a fresh loopback port; returns the
/// base URL and the captured request bodies (form-encoded, in order).
fn serve(script: Vec<(u16, String)>) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let bodies = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&bodies);
    std::thread::spawn(move || {
        for (status, json) in script {
            let Ok((mut sock, _)) = listener.accept() else {
                return;
            };
            let mut buf = Vec::new();
            let mut chunk = [0u8; 1024];
            // Read until the headers end, then exactly Content-Length more.
            let body_len = loop {
                let n = sock.read(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break 0;
                }
                buf.extend_from_slice(&chunk[..n]);
                if let Some(head_end) = find(&buf, b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&buf[..head_end]).to_lowercase();
                    let want: usize = head
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length:"))
                        .and_then(|v| v.trim().parse().ok())
                        .unwrap_or(0);
                    while buf.len() < head_end + 4 + want {
                        let n = sock.read(&mut chunk).unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        buf.extend_from_slice(&chunk[..n]);
                    }
                    break head_end + 4;
                }
            };
            seen.lock()
                .unwrap()
                .push(String::from_utf8_lossy(&buf[body_len..]).to_string());
            let reply = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\n\
                 content-length: {}\r\nconnection: close\r\n\r\n{json}",
                json.len()
            );
            let _ = sock.write_all(reply.as_bytes());
        }
    });
    (base, bodies)
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn endpoints(base: &str) -> DeviceEndpoints {
    DeviceEndpoints {
        device_url: format!("{base}/device"),
        token_url: format!("{base}/token"),
        client_id: "cid-123".into(),
        scope: "openid model.completion".into(),
    }
}

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

#[test]
fn start_sends_the_grant_and_parses_the_card() {
    let (base, bodies) = serve(vec![(
        200,
        r#"{"device_code":"dev-sec-1","user_code":"WDJB-MJHT","verification_uri":"https://x/activate","verification_uri_complete":"https://x/activate?user_code=WDJB-MJHT","interval":5,"expires_in":900}"#.into(),
    )]);
    let got = block_on(device_start(&endpoints(&base), Some("CHAL"))).unwrap();
    assert_eq!(got.device_code, "dev-sec-1");
    assert_eq!(got.user_code, "WDJB-MJHT");
    assert_eq!(got.verification_uri, "https://x/activate");
    assert_eq!(got.interval, 5);
    assert_eq!(got.expires_in, 900);
    let body = bodies.lock().unwrap()[0].clone();
    assert!(body.contains("client_id=cid-123"), "{body}");
    assert!(body.contains("scope=openid+model.completion"), "{body}");
    assert!(body.contains("code_challenge=CHAL"), "{body}");
    assert!(body.contains("code_challenge_method=S256"), "{body}");
}

#[test]
fn poll_maps_every_rfc8628_verdict_in_order() {
    let err = |e: &str| (400, format!(r#"{{"error":"{e}"}}"#));
    let (base, bodies) = serve(vec![
        err("authorization_pending"),
        err("slow_down"),
        err("expired_token"),
        err("access_denied"),
        (
            200,
            r#"{"access_token":"at-9","refresh_token":"rt-9","expires_in":3600,"resource_url":"https://portal.example/v1"}"#.into(),
        ),
    ]);
    let e = endpoints(&base);
    let mut verdicts = Vec::new();
    for _ in 0..5 {
        verdicts.push(block_on(device_poll(&e, "dev-sec-1", Some("VERIF"))).unwrap());
    }
    assert!(matches!(verdicts[0], DevicePoll::Pending), "{verdicts:?}");
    assert!(matches!(verdicts[1], DevicePoll::SlowDown), "{verdicts:?}");
    assert!(matches!(verdicts[2], DevicePoll::Expired), "{verdicts:?}");
    assert!(matches!(verdicts[3], DevicePoll::Denied), "{verdicts:?}");
    match &verdicts[4] {
        DevicePoll::Ready(t) => {
            assert_eq!(t.access_token, "at-9");
            assert_eq!(t.refresh_token.as_deref(), Some("rt-9"));
            assert_eq!(t.expires_in, Some(3600));
            assert_eq!(t.resource_url.as_deref(), Some("https://portal.example/v1"));
        }
        other => panic!("wanted Ready, got {other:?}"),
    }
    let body = bodies.lock().unwrap()[0].clone();
    assert!(
        body.contains("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code"),
        "{body}"
    );
    assert!(body.contains("device_code=dev-sec-1"), "{body}");
    assert!(body.contains("code_verifier=VERIF"), "{body}");
}

#[test]
fn refresh_trades_the_token_and_names_status_only_on_failure() {
    let (base, bodies) = serve(vec![
        (
            200,
            r#"{"access_token":"at-new","refresh_token":"rt-new","expires_in":7200}"#.into(),
        ),
        (400, r#"{"error":"invalid_grant"}"#.into()),
    ]);
    let e = endpoints(&base);
    let ok = block_on(device_refresh(&e, "rt-old-secret")).unwrap();
    assert_eq!(ok.access_token, "at-new");
    assert_eq!(ok.refresh_token.as_deref(), Some("rt-new"));
    let body = bodies.lock().unwrap()[0].clone();
    assert!(body.contains("grant_type=refresh_token"), "{body}");
    let err = block_on(device_refresh(&e, "rt-old-secret")).unwrap_err();
    let text = format!("{err:#}");
    assert!(text.contains("400"), "{text}");
    // The refresh token must never ride in an error message.
    assert!(!text.contains("rt-old-secret"), "{text}");
}

/// The redaction rule, pinned: a `TokenSet` in any panic/debug output shows
/// no token material.
#[test]
fn a_token_set_debug_prints_no_secret() {
    let t = TokenSet {
        access_token: "at-secret".into(),
        refresh_token: Some("rt-secret".into()),
        expires_in: Some(60),
        resource_url: None,
    };
    let dbg = format!("{t:?}");
    assert!(!dbg.contains("at-secret"), "{dbg}");
    assert!(!dbg.contains("rt-secret"), "{dbg}");
    assert!(dbg.contains("<redacted>"), "{dbg}");
}
