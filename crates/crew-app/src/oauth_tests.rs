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
    // The nonce is what keeps requests that are not part of THIS flow — a
    // probe, another crew window's callback — from ending the wait. It is not
    // a secret from other local processes (see `parse_request`'s doc: the URL
    // is visible in `ps`), so it is not what makes the flow unspoofable.
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
    // Through `run_listener`, not a raw `join()`: a join has no timeout, so
    // the very regressions the tests below guard against would hang cargo
    // here instead of failing it.
    let done = run_listener(listener, TEST_TIMEOUT);
    // A wrong-nonce request must not end the wait...
    let _ = ureq_like_get(port, "/wrong?code=nope");
    let _ = ureq_like_get(port, "/n0nce?code=the-code");
    assert_eq!(
        done.recv_timeout(TEST_TIMEOUT).expect("listener stuck"),
        Waited::Code("the-code".into())
    );
}

#[test]
fn the_listener_gives_up_rather_than_waiting_forever() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let started = std::time::Instant::now();
    let got = await_callback(listener, "n0nce", std::time::Duration::from_millis(300));
    // A plain deadline, not a broken listener — the two must not read alike,
    // or a half-working bind tells the user they were too slow.
    assert_eq!(got, Waited::TimedOut);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "did not time out"
    );
}

#[test]
fn a_peer_that_connects_and_says_nothing_does_not_pin_the_listener() {
    // Any local process can reach this port. Holding the connection open and
    // sending nothing used to park the worker thread in `read_line` forever
    // — past the flow timeout, holding the port, surviving a cancel.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let done = run_listener_within(listener, TEST_TIMEOUT, TEST_CONN_BUDGET);
    // Connect first, so this silent peer is the one `accept` hands back
    // first, and keep it open for the whole test.
    let _silent = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    let _ = ureq_like_get(port, "/n0nce?code=the-code");
    // Bounded wait: on regression this fails instead of hanging cargo.
    assert_eq!(
        done.recv_timeout(TEST_TIMEOUT).expect("listener stuck"),
        Waited::Code("the-code".into()),
        "the real callback must still land behind a silent peer"
    );
}

#[test]
fn an_endless_request_line_is_not_accumulated_without_bound() {
    // A peer sending bytes with no newline would otherwise grow the buffer
    // for as long as it kept sending.
    let flood = vec![b'x'; 4 * MAX_REQUEST_BYTES as usize];
    let got = read_bounded(&flood[..], far_off(), |_| {});
    assert!(
        got.len() as u64 <= MAX_REQUEST_BYTES,
        "read {} bytes, cap is {MAX_REQUEST_BYTES}",
        got.len()
    );
}

#[test]
fn the_read_stops_at_the_end_of_the_request_line() {
    // The chunked read can pull headers in with the request line; only the
    // line is ours.
    let req = b"GET /n0nce?code=abc HTTP/1.1\r\nHost: localhost\r\n\r\n";
    let got = read_bounded(&req[..], far_off(), |_| {});
    assert_eq!(got, "GET /n0nce?code=abc HTTP/1.1\r\n");
    match parse_request(got.trim_end(), "n0nce") {
        Callback::Code(c) => assert_eq!(c, "abc"),
        other => panic!("expected a code, got {other:?}"),
    }
}

#[test]
fn a_dribbling_peer_is_abandoned_at_the_deadline_not_at_the_byte_cap() {
    // One byte at a time, never a newline, never idle long enough to trip a
    // per-`read` timeout — the shape that a byte cap and a per-read deadline
    // both fail to bound. Only a wall-clock deadline for the connection ends
    // this; without one it runs to MAX_REQUEST_BYTES, which at this rate is
    // over eight seconds here and over four HOURS at the real timeouts.
    struct Dribble;
    impl std::io::Read for Dribble {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            std::thread::sleep(std::time::Duration::from_millis(1));
            buf[0] = b'x';
            Ok(1)
        }
    }
    let started = std::time::Instant::now();
    let deadline = started + std::time::Duration::from_millis(150);
    let got = read_bounded(Dribble, deadline, |_| {});
    assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "the connection ran past its deadline ({:?})",
        started.elapsed()
    );
    assert!(
        (got.len() as u64) < MAX_REQUEST_BYTES,
        "stopped at the byte cap, not the deadline: {} bytes",
        got.len()
    );
}

#[test]
fn a_dribbling_peer_does_not_park_the_listener_over_a_real_socket() {
    // The same shape over a real connection: it must be dropped at the
    // connection deadline and the real callback must land behind it.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let done = run_listener_within(listener, TEST_TIMEOUT, TEST_CONN_BUDGET);
    let mut slow = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = stop.clone();
    let dribbler = std::thread::spawn(move || {
        use std::io::Write;
        // Never a newline, and paced well inside READ_TIMEOUT. Ends as soon
        // as the listener drops us (write fails) or the test says so.
        while !flag.load(std::sync::atomic::Ordering::Relaxed) {
            if slow.write_all(b"x").is_err() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    });
    let _ = ureq_like_get(port, "/n0nce?code=the-code");
    // Bounded: on regression this fails at TEST_TIMEOUT instead of parking
    // cargo for the ~2.7 minutes 8192 dribbled bytes would take.
    let got = done.recv_timeout(TEST_TIMEOUT);
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = dribbler.join();
    assert_eq!(
        got.expect("the listener parked on a dribbling peer"),
        Waited::Code("the-code".into()),
        "the real callback must still land behind a dribbling peer"
    );
}

#[test]
fn a_flooding_peer_does_not_stop_the_real_callback_landing() {
    // The same cap over a real socket: the flood is answered and dropped,
    // and the wait continues.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let done = run_listener_within(listener, TEST_TIMEOUT, TEST_CONN_BUDGET);
    let mut flood = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    // Detached, and never joined: once the listener has taken its capped
    // read it drops the stream, so this write is expected to fail part-way.
    std::thread::spawn(move || {
        use std::io::Write;
        let chunk = vec![b'x'; 4096];
        for _ in 0..64 {
            if flood.write_all(&chunk).is_err() {
                return;
            }
        }
    });
    let _ = ureq_like_get(port, "/n0nce?code=the-code");
    assert_eq!(
        done.recv_timeout(TEST_TIMEOUT).expect("listener stuck"),
        Waited::Code("the-code".into())
    );
}

#[test]
fn remote_supplied_text_is_clamped_before_it_reaches_a_note() {
    // An `error=` value comes from the callback URL: unbounded, and not ours
    // to print in full.
    let long = "e".repeat(5_000);
    let short = short_reason(&long);
    assert!(short.chars().count() <= MAX_REASON_CHARS + 1, "{short}");
    assert!(short.ends_with('…'), "truncation is visible: {short}");
    // Multi-byte text must not be cut mid-character (that would panic).
    assert_eq!(short_reason(&"é".repeat(200)).chars().count(), 101);
    // Short reasons pass through untouched.
    assert_eq!(short_reason("access_denied"), "access_denied");
}

#[test]
fn the_debug_form_never_carries_the_authorization_code() {
    // The tests above format these types in their failure messages; the code
    // must not ride along into a panic message or anywhere else.
    let printed = format!(
        "{:?} {:?}",
        Callback::Code("the-code".into()),
        Waited::Code("the-code".into())
    );
    assert!(!printed.contains("the-code"), "{printed}");
    assert!(printed.contains("redacted"), "{printed}");
    // A denial is remote text, not a secret — keep it printable.
    assert!(format!("{:?}", Waited::Denied("access_denied".into())).contains("access_denied"));
}

const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
/// The per-connection budget the socket tests run with — the real
/// [`CONN_TIMEOUT`] shape, scaled down so an abandoned peer costs the suite
/// milliseconds rather than seconds.
const TEST_CONN_BUDGET: std::time::Duration = std::time::Duration::from_millis(200);

/// A deadline far enough out that a test is exercising the byte cap, not it.
fn far_off() -> std::time::Instant {
    std::time::Instant::now() + TEST_TIMEOUT
}

/// Run `await_callback` on its own thread, handing back a channel rather than
/// a `JoinHandle`: a `join()` has no timeout, so a regression here would hang
/// cargo forever instead of failing.
fn run_listener(
    listener: std::net::TcpListener,
    timeout: std::time::Duration,
) -> std::sync::mpsc::Receiver<Waited> {
    run_listener_within(listener, timeout, CONN_TIMEOUT)
}

/// [`run_listener`] with the per-connection budget pinned.
fn run_listener_within(
    listener: std::net::TcpListener,
    timeout: std::time::Duration,
    conn: std::time::Duration,
) -> std::sync::mpsc::Receiver<Waited> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(await_callback_within(listener, "n0nce", timeout, conn));
    });
    rx
}

/// Minimal HTTP GET over a raw socket — the app has no HTTP client and this
/// test needs no dependency to make one request.
fn ureq_like_get(port: u16, target: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port))?;
    write!(s, "GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n")?;
    Ok(())
}
