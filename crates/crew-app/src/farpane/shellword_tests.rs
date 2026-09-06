use super::{escape, token_start, unescape};

#[test]
fn token_starts_after_the_last_unescaped_space() {
    assert_eq!(token_start("ls"), 0);
    assert_eq!(token_start("ls src/fa"), 3);
    assert_eq!(
        token_start("cd My\\ Folder/"),
        3,
        "escaped space is inside the word"
    );
    assert_eq!(token_start("cp a\\ b c"), 8);
    assert_eq!(token_start("ls "), 3);
}

#[test]
fn unescape_drops_backslashes_and_a_matching_quote_pair() {
    assert_eq!(unescape("My\\ Folder"), "My Folder");
    assert_eq!(unescape("\"My Folder\""), "My Folder");
    assert_eq!(unescape("'it''s'"), "it''s");
    assert_eq!(
        unescape("\"half"),
        "\"half",
        "an unmatched quote is literal"
    );
    assert_eq!(unescape("\""), "\"", "a lone quote is not a pair");
    assert_eq!(unescape("plain"), "plain");
    assert_eq!(
        unescape("trail\\"),
        "trail\\",
        "a trailing backslash survives"
    );
}

#[test]
fn escape_round_trips_through_unescape() {
    for name in [
        "My Folder",
        "a&b",
        "it's",
        "tab\there",
        "$HOME",
        "x(y)",
        "plain",
    ] {
        assert_eq!(unescape(&escape(name)), name, "{name}");
    }
    assert_eq!(escape("My Folder"), "My\\ Folder");
    assert_eq!(
        escape("plain-name_1"),
        "plain-name_1",
        "safe names are untouched"
    );
}
