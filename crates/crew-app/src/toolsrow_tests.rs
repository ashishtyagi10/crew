use super::*;

#[test]
fn a_long_tool_name_is_cut_in_the_middle_so_both_ends_survive() {
    // Both halves tell two calls apart; a head or tail clip throws one away.
    let out = fit("google_workspace:search_gmail_messages", 20);
    assert_eq!(out.chars().count(), 20, "{out:?}");
    assert!(out.starts_with("google"), "{out:?}");
    assert!(out.ends_with("messages"), "{out:?}");
    assert!(out.contains('\u{2026}'), "{out:?}");
    // A name that fits is untouched.
    assert_eq!(fit("sys:run", 20), "sys:run");
}

/// A note is free text of unknown length and the detail line is the only
/// place it is ever shown, so clipping would drop exactly the message the
/// line exists to carry.
#[test]
fn a_long_note_wraps_rather_than_losing_its_tail() {
    let lines = wrap("nobody answered the approval in five whole minutes", 20);
    assert!(lines.iter().all(|l| l.chars().count() <= 20), "{lines:?}");
    assert_eq!(
        lines.join(" "),
        "nobody answered the approval in five whole minutes"
    );
}

#[test]
fn a_word_longer_than_the_line_is_broken_rather_than_overflowing() {
    let lines = wrap("aaaaaaaaaaaaaaaaaaaaaaaaaaaa short", 10);
    assert!(lines.iter().all(|l| l.chars().count() <= 10), "{lines:?}");
    assert!(lines.last().unwrap().contains("short"), "{lines:?}");
}
