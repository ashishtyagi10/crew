use super::*;

fn guard() -> crate::app::ThemeGuard {
    crate::app::theme_test_guard()
}

#[test]
fn a_real_command_a_prefix_of_one_and_a_typo_are_three_different_things() {
    assert_eq!(classify("/theme"), Cmd::Known);
    assert_eq!(classify("/them"), Cmd::Partial);
    assert_eq!(classify("/zzz"), Cmd::Unknown);
    assert_eq!(classify("/"), Cmd::Partial, "an empty slash is mid-typing");
}

/// Commands are typed in whatever case, and `/THEME` runs.
#[test]
fn classification_ignores_case() {
    assert_eq!(classify("/THEME"), Cmd::Known);
    assert_eq!(classify("/Them"), Cmd::Partial);
}

/// The whole point: the bar says a command is not going to work before you
/// press Enter, and the three states are three colours.
#[test]
fn the_leading_command_takes_a_colour_of_its_own() {
    let _g = guard();
    let t = crew_theme::theme();
    let head = |s: &str| paint(s)[0];
    assert_eq!(head("/theme dark"), crate::palette::accent());
    assert_eq!(head("/them"), t.text_muted);
    assert_eq!(head("/zzz"), t.bell);
    assert_ne!(head("/theme"), head("/zzz"), "a typo reads as a command");
}

/// Only the command word is coloured — its argument is ordinary text, or the
/// whole line would read as one token.
#[test]
fn the_argument_after_a_command_is_ordinary_text() {
    let _g = guard();
    let p = paint("/theme dark");
    let ink = crew_theme::theme().ink;
    assert_eq!(p[0], crate::palette::accent());
    assert_eq!(p["/theme ".len()], ink, "the argument was painted too");
}

/// Bare text is not a command and gets no command colour anywhere in it, even
/// when a slash appears later.
#[test]
fn text_that_does_not_begin_with_a_slash_is_left_alone() {
    let _g = guard();
    let ink = crew_theme::theme().ink;
    let p = paint("ls src/main.rs");
    assert!(p.iter().all(|&c| c == ink), "bare text was classified");
}

#[test]
fn flags_recede_and_only_at_the_start_of_a_word() {
    let _g = guard();
    let t = crew_theme::theme();
    let s = "grep -rn thing a-b";
    let p = paint(s);
    let at = |sub: &str| p[s.find(sub).unwrap()];
    assert_eq!(at("-rn"), t.dim);
    assert_eq!(at("thing"), t.ink, "the argument was dimmed with the flag");
    assert_eq!(at("a-b"), t.ink, "a hyphen inside a word is not a flag");
}

#[test]
fn a_quoted_run_is_marked_from_its_opening_quote_to_its_closing_one() {
    let _g = guard();
    let string = crate::chatink::token_fg(crate::md::syntax::Token::Str);
    let s = "/find \"two words\" after";
    let p = paint(s);
    let open = s.find('"').unwrap();
    let close = s.rfind('"').unwrap();
    assert_eq!(p[open], string, "the opening quote is not marked");
    assert_eq!(p[close], string, "the closing quote is not marked");
    assert!(
        (open..=close).all(|i| p[i] == string),
        "the middle of the quoted run is unmarked"
    );
    assert_ne!(p[close + 2], string, "the marking ran past the quote");
}

/// An unterminated quote marks to the end of the line — which is what tells
/// you it is unterminated.
#[test]
fn an_unclosed_quote_runs_to_the_end() {
    let _g = guard();
    let string = crate::chatink::token_fg(crate::md::syntax::Token::Str);
    let p = paint("echo \"still open");
    assert!(p[5..].iter().all(|&c| c == string));
}

/// One colour per character, whatever is in the text — the renderer zips the
/// two together and a short list would silently drop the tail.
#[test]
fn every_character_gets_exactly_one_colour() {
    let _g = guard();
    for s in ["", "/", "/theme dark", "a\u{1f600}b", "\"", "--", "  "] {
        assert_eq!(paint(s).len(), s.chars().count(), "{s:?}");
    }
}
