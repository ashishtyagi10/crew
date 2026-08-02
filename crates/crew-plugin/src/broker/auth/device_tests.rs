//! The flow engine against a scripted local OAuth server (loopback
//! `TcpListener`, one `(status, json)` response per request) with the
//! sleeper and storage injected — every timing rule asserted as NUMBERS
//! (recorded sleep durations), never a verdict.
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

fn start_json(interval: u64, expires_in: u64) -> (u16, String) {
    (
        200,
        format!(
            r#"{{"device_code":"dev-1","user_code":"WDJB-MJHT","verification_uri":"https://x/go","interval":{interval},"expires_in":{expires_in}}}"#
        ),
    )
}

fn pending() -> (u16, String) {
    (400, r#"{"error":"authorization_pending"}"#.into())
}

fn ready() -> (u16, String) {
    (
        200,
        r#"{"access_token":"at-1","refresh_token":"rt-1","expires_in":3600,"resource_url":"https://portal/v1"}"#.into(),
    )
}

fn ends(base: &str) -> DeviceEndpoints {
    endpoints_from(
        &registry::by_name("dashscope").unwrap().device.unwrap(),
        Some(base.to_string()),
    )
}

/// A driven flow: (outcome, sleeps in secs, stored grants, rendered cards).
type Driven = (Outcome, Vec<u64>, Vec<(String, StoredToken)>, Vec<String>);

/// Run a flow recording sleeps (as whole seconds) and stored grants.
fn drive(script: Vec<(u16, String)>) -> Driven {
    let base = serve(script);
    let mut sleeps = Vec::new();
    let mut stored = Vec::new();
    let mut cards = Vec::new();
    let out = run_flow(
        "prov-under-test",
        &ends(&base),
        &mut |c| cards.push(format!("{} {}", c.user_code, c.verification_uri)),
        &mut |d| sleeps.push(d.as_secs()),
        &mut |p, t| stored.push((p.to_string(), t)),
        &|| false,
    );
    (out, sleeps, stored, cards)
}

#[test]
fn the_happy_path_polls_at_the_interval_and_stores_the_grant() {
    let (out, sleeps, stored, cards) = drive(vec![start_json(1, 900), pending(), ready()]);
    assert_eq!(out, Outcome::SignedIn);
    assert_eq!(sleeps, [1, 1], "one pending poll then the grant");
    assert_eq!(cards, ["WDJB-MJHT https://x/go"]);
    assert_eq!(stored.len(), 1);
    let (p, t) = &stored[0];
    assert_eq!(p, "prov-under-test");
    assert_eq!(t.access, "at-1");
    assert_eq!(t.refresh.as_deref(), Some("rt-1"));
    assert_eq!(t.resource.as_deref(), Some("https://portal/v1"));
}

#[test]
fn slow_down_adds_exactly_five_seconds_to_every_later_poll() {
    let script = vec![
        start_json(1, 900),
        (400, r#"{"error":"slow_down"}"#.into()),
        pending(),
        ready(),
    ];
    let (out, sleeps, ..) = drive(script);
    assert_eq!(out, Outcome::SignedIn);
    assert_eq!(sleeps, [1, 6, 6], "1s, then slow_down makes it 1+5");
}

#[test]
fn an_expired_device_code_ends_the_flow_with_nothing_stored() {
    let (out, sleeps, stored, _) = drive(vec![
        start_json(1, 900),
        (400, r#"{"error":"expired_token"}"#.into()),
    ]);
    assert_eq!(out, Outcome::Expired);
    assert_eq!(sleeps, [1]);
    assert_eq!(stored.len(), 0, "an expired flow must store nothing");
}

#[test]
fn denial_ends_the_flow_with_nothing_stored() {
    let (out, _, stored, _) = drive(vec![
        start_json(1, 900),
        (400, r#"{"error":"access_denied"}"#.into()),
    ]);
    assert_eq!(out, Outcome::Denied);
    assert_eq!(stored.len(), 0);
}

/// The wait is bounded by the server's `expires_in`: interval 60 against a
/// 120s lifetime is exactly two polls, then timeout — never a third.
#[test]
fn the_total_wait_is_bounded_by_the_device_code_lifetime() {
    let (out, sleeps, stored, _) = drive(vec![start_json(60, 120), pending(), pending()]);
    assert_eq!(out, Outcome::TimedOut);
    assert_eq!(sleeps, [60, 60], "exactly two polls fit a 120s lifetime");
    assert_eq!(stored.len(), 0);
}

/// …and by [`MAX_WAIT_SECS`] whatever the server claims.
#[test]
fn the_budget_caps_the_servers_word_at_max_wait() {
    assert_eq!(poll_budget(100_000), 900);
    assert_eq!(poll_budget(120), 120);
}

/// `/stop` between polls: the second iteration sees the cancel flag and the
/// flow ends `Stopped` after EXACTLY one poll — no further HTTP, no grant.
#[test]
fn cancellation_stops_the_flow_between_polls() {
    let base = serve(vec![start_json(1, 900), pending()]);
    let sleeps = std::cell::RefCell::new(Vec::new());
    let out = run_flow(
        "prov-cancel",
        &ends(&base),
        &mut |_| {},
        &mut |d| sleeps.borrow_mut().push(d.as_secs()),
        &mut |_, _| panic!("a cancelled flow must store nothing"),
        &|| !sleeps.borrow().is_empty(),
    );
    assert_eq!(out, Outcome::Stopped);
    assert_eq!(
        *sleeps.borrow(),
        [1],
        "exactly one poll before the cancel lands"
    );
}
