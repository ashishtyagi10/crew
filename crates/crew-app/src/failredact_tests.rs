use super::redact;

#[test]
fn a_flag_with_a_secret_name_loses_its_value_either_way_it_is_given() {
    assert_eq!(
        redact("deploy --token=abc123 --prod"),
        "deploy --token=*** --prod"
    );
    assert_eq!(redact("cli --api-key abc123 run"), "cli --api-key *** run");
    assert_eq!(redact("x --Password 'a b c' y"), "x --Password *** y");
    assert_eq!(redact("x --client-secret=\"a b\""), "x --client-secret=***");
    assert_eq!(
        redact("x --db-passwd=p --credentials=c"),
        "x --db-passwd=*** --credentials=***"
    );
}

#[test]
fn a_secret_flag_with_no_value_does_not_eat_the_next_flag() {
    assert_eq!(redact("tool --auth --verbose"), "tool --auth --verbose");
}

#[test]
fn an_authorization_header_and_a_bearer_token_are_masked() {
    assert_eq!(
        redact(r#"curl -H "Authorization: Bearer sk-live-9" https://x.io"#),
        r#"curl -H "Authorization: ***" https://x.io"#
    );
    assert_eq!(
        redact("curl -H 'authorization: abc' u"),
        "curl -H 'authorization: ***' u"
    );
    assert_eq!(
        redact("curl --oauth2-bearer x Bearer tok123"),
        "curl --oauth2-bearer *** Bearer ***"
    );
}

#[test]
fn an_environment_assignment_with_a_secret_name_is_masked() {
    assert_eq!(redact("API_KEY=abc cargo run"), "API_KEY=*** cargo run");
    assert_eq!(
        redact("GITHUB_TOKEN=\"a b\" gh pr list"),
        "GITHUB_TOKEN=*** gh pr list"
    );
    assert_eq!(
        redact("DB_PASSWORD=x SECRET=y make"),
        "DB_PASSWORD=*** SECRET=*** make"
    );
    assert_eq!(redact("monkey=3 PATH=/bin ls"), "monkey=3 PATH=/bin ls");
}

#[test]
fn url_userinfo_is_masked_and_the_host_kept() {
    assert_eq!(
        redact("git clone https://me:hunter2@github.com/a/b.git"),
        "git clone https://***@github.com/a/b.git"
    );
    assert_eq!(redact("curl https://tok@host/x"), "curl https://***@host/x");
}

#[test]
fn a_clean_command_is_unchanged() {
    for c in [
        "cargo test -p crew-app --no-fail-fast",
        "git commit -m 'fix the keyboard'",
        "curl -H 'Accept: json' https://example.com/a@b",
        "ls -la ~/code",
    ] {
        assert_eq!(redact(c), c);
    }
}
