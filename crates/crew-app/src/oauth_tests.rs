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
