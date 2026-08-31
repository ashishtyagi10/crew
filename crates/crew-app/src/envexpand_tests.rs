use super::*;

#[test]
fn expands_simple_and_braced() {
    std::env::set_var("CREW_EE_A", "/tmp/a");
    assert_eq!(expand_env("$CREW_EE_A/x"), "/tmp/a/x");
    assert_eq!(expand_env("${CREW_EE_A}y"), "/tmp/ay");
    assert_eq!(expand_env("pre $CREW_EE_A post"), "pre /tmp/a post");
}

#[test]
fn unset_is_empty_and_literals_kept() {
    std::env::remove_var("CREW_EE_UNSET");
    assert_eq!(expand_env("a${CREW_EE_UNSET}b"), "ab");
    assert_eq!(expand_env("no vars here"), "no vars here");
    assert_eq!(expand_env("$"), "$");
    assert_eq!(expand_env("~/plain"), "~/plain");
    // `$5` is not a variable (names can't start with a digit).
    assert_eq!(expand_env("cost $5"), "cost $5");
}

/// `expand_env` walks raw bytes with `s[i..]` slices, so non-ASCII input and
/// truncated tokens are the panic-prone cases. These pin the no-panic /
/// literal-passthrough behaviour for a hand-rolled UTF-8 parser fed user
/// input (the input bar).
#[test]
fn utf8_and_truncated_tokens_stay_literal_without_panicking() {
    // `$` before a multibyte char: not a valid name start, kept literal.
    assert_eq!(expand_env("$é"), "$é");
    assert_eq!(expand_env("price 5€ each"), "price 5€ each");
    // `$` at end of string.
    assert_eq!(expand_env("a$"), "a$");
    // Unterminated / empty braces are literal.
    assert_eq!(expand_env("${unterminated"), "${unterminated");
    assert_eq!(expand_env("${}x"), "${}x");
    // A braced name with multibyte chars parses and (being unset) expands to
    // empty — exercising a non-ASCII name slice without panicking.
    std::env::remove_var("☃");
    assert_eq!(expand_env("x${☃}y"), "xy");
}
