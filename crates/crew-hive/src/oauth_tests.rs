use super::*;

/// RFC 7636 Appendix B's S256 vector. This pins the ENCODING (base64url,
/// no padding, `-`/`_` not `+`/`/`) against an external authority rather than
/// against our own implementation — a self-consistent test would pass just as
/// happily with standard base64, which OpenRouter would reject.
#[test]
fn the_challenge_matches_the_rfc_7636_vector() {
    assert_eq!(
        challenge_for("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
}

#[test]
fn a_verifier_is_long_url_safe_and_unpredictable() {
    let a = pkce();
    let b = pkce();
    assert_eq!(a.verifier.chars().count(), 64);
    assert!(
        a.verifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "verifier must be URL-safe: {}",
        a.verifier
    );
    assert_ne!(
        a.verifier, b.verifier,
        "two flows must not share a verifier"
    );
    assert_eq!(a.challenge, challenge_for(&a.verifier));
}

#[test]
fn a_challenge_carries_no_padding_or_unsafe_characters() {
    let p = pkce();
    assert!(!p.challenge.contains('='), "no padding: {}", p.challenge);
    assert!(
        !p.challenge.contains('+'),
        "url-safe alphabet: {}",
        p.challenge
    );
    assert!(
        !p.challenge.contains('/'),
        "url-safe alphabet: {}",
        p.challenge
    );
}

#[test]
fn the_authorize_url_carries_exactly_the_documented_parameters() {
    let url = authorize_url("http://127.0.0.1:8731/abc", "CHAL");
    assert!(url.starts_with("https://openrouter.ai/auth?"), "{url}");
    assert!(url.contains("code_challenge=CHAL"), "{url}");
    assert!(url.contains("code_challenge_method=S256"), "{url}");
    // The callback must be escaped, or the `:` and `/` truncate the parameter.
    assert!(
        url.contains("callback_url=http%3A%2F%2F127.0.0.1%3A8731%2Fabc"),
        "{url}"
    );
    // No client_id: OpenRouter requires no app registration, and inventing one
    // would break the flow.
    assert!(!url.contains("client_id"), "{url}");
}
