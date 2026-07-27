use super::*;

#[test]
fn a_matching_path_with_a_code_yields_it() {
    match parse_request("GET /n0nce?code=abc123 HTTP/1.1", "n0nce") {
        Callback::Code(c) => assert_eq!(c, "abc123"),
        other => panic!("expected a code, got {other:?}"),
    }
}

#[test]
fn a_wrong_nonce_is_ignored() {
    // Any local process can reach the port. The nonce is what stops another
    // one feeding us a code — the loopback equivalent of an OAuth `state`.
    assert!(matches!(
        parse_request("GET /guessed?code=abc123 HTTP/1.1", "n0nce"),
        Callback::Ignore
    ));
}

#[test]
fn a_denial_is_reported_not_swallowed() {
    match parse_request("GET /n0nce?error=access_denied HTTP/1.1", "n0nce") {
        Callback::Denied(e) => assert_eq!(e, "access_denied"),
        other => panic!("expected a denial, got {other:?}"),
    }
}

#[test]
fn junk_and_empty_queries_are_ignored() {
    for line in [
        "GET /n0nce HTTP/1.1",
        "GET /n0nce?code= HTTP/1.1",
        "GET / HTTP/1.1",
        "",
        "not an http request at all",
        "POST /n0nce?code=abc HTTP/1.1",
    ] {
        assert!(
            matches!(parse_request(line, "n0nce"), Callback::Ignore),
            "should ignore: {line:?}"
        );
    }
}

#[test]
fn extra_parameters_around_the_code_do_not_confuse_it() {
    match parse_request("GET /n0nce?state=x&code=abc&scope=y HTTP/1.1", "n0nce") {
        Callback::Code(c) => assert_eq!(c, "abc"),
        other => panic!("expected a code, got {other:?}"),
    }
}

#[test]
fn the_listener_returns_the_code_from_a_matching_request() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let h = std::thread::spawn(move || await_callback(listener, "n0nce", TEST_TIMEOUT));
    // A wrong-nonce request must not end the wait...
    let _ = ureq_like_get(port, "/wrong?code=nope");
    let _ = ureq_like_get(port, "/n0nce?code=the-code");
    assert_eq!(h.join().unwrap(), Callback::Code("the-code".into()));
}

#[test]
fn the_listener_gives_up_rather_than_waiting_forever() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let started = std::time::Instant::now();
    let got = await_callback(listener, "n0nce", std::time::Duration::from_millis(300));
    assert!(matches!(got, Callback::Ignore), "{got:?}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "did not time out"
    );
}

const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Minimal HTTP GET over a raw socket — the app has no HTTP client and this
/// test needs no dependency to make one request.
fn ureq_like_get(port: u16, target: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port))?;
    write!(s, "GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n")?;
    Ok(())
}
