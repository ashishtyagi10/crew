use super::*;
use crate::ipc_types::PROTOCOL_V;

#[test]
fn authorized_requires_federation_on_and_an_exact_token() {
    // Federation off (no configured token) → everything rejected.
    assert!(!authorized(None, "anything"));
    assert!(!authorized(Some(""), "anything"));
    // On → only the exact token passes.
    assert!(authorized(Some("s3cret"), "s3cret"));
    assert!(!authorized(Some("s3cret"), "s3cre"));
    assert!(!authorized(Some("s3cret"), "s3crett"));
    assert!(!authorized(Some("s3cret"), "wrong!"));
    assert!(!authorized(Some("s3cret"), ""));
}

#[test]
fn relay_frame_round_trips_the_envelope() {
    // The dialer's borrowed frame serializes to the same JSON the listener
    // deserializes into an owned RelayRequest — the ipc Request rides inside
    // unchanged.
    let req = Request::Ask {
        v: PROTOCOL_V,
        from: "builder".into(),
        to: "schema".into(),
        question: "which API?".into(),
        id: "q1".into(),
    };
    let wire = serde_json::to_string(&RelayRequestRef {
        token: "tok",
        instance: Some("alpha"),
        req: &req,
    })
    .unwrap();
    let got: RelayRequest = serde_json::from_str(&wire).unwrap();
    assert_eq!(got.token, "tok");
    assert_eq!(got.instance.as_deref(), Some("alpha"));
    assert_eq!(got.req, req);
}

#[test]
fn relay_rejects_a_bad_token_over_tcp() {
    // A real loopback round-trip through serve_conn: a wrong token gets the
    // generic Unreachable, never a distinct "bad token" signal, and no bridge
    // to any local socket is attempted.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        if let Ok((s, _)) = listener.accept() {
            super::serve_conn(s, Some("right"));
        }
    });
    let req = Request::Panes { v: PROTOCOL_V };
    let reply = dial(&addr.ip().to_string(), addr.port(), None, &req, "wrong");
    assert!(matches!(
        reply,
        Some(Reply::NoAnswer {
            reason: NoAnswer::Unreachable,
            ..
        })
    ));
}
