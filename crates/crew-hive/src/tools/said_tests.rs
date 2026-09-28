use super::*;

#[test]
fn a_reply_that_was_only_the_call_adds_nothing() {
    assert_eq!(said(""), "");
    assert_eq!(said("  \n\n "), "");
}

#[test]
fn the_message_is_headed_and_ends_where_the_called_line_starts() {
    assert_eq!(
        said("\nThe bug is in clip(); reading it next.\n\n"),
        "YOUR MESSAGE:\nThe bug is in clip(); reading it next.\n"
    );
}

#[test]
fn a_long_message_is_cut_at_a_line_end_and_says_so() {
    let line = "x".repeat(99);
    let text = [line.as_str(); 12].join("\n"); // 1,199 chars
    let out = said(&text);
    let body = out
        .strip_prefix("YOUR MESSAGE:\n")
        .expect("heading")
        .trim_end();
    let (kept, note) = body.rsplit_once('\n').expect("a note line");
    // Eight whole 100-char lines fit in 800; the ninth would not.
    assert_eq!(kept, [line.as_str(); 8].join("\n"));
    assert!(kept.lines().all(|l| l == line), "a line was cut mid-way");
    assert_eq!(note, "[\u{2026} 400 more chars of this message not shown]");
}

#[test]
fn one_line_longer_than_the_clip_is_cut_at_the_clip() {
    let text = "é".repeat(SAID_CLIP + 50);
    let out = clip_lines(&text, SAID_CLIP);
    let (kept, note) = out.split_once('\n').unwrap();
    assert_eq!(kept.chars().count(), SAID_CLIP);
    assert!(note.contains("50 more chars"), "{note}");
}
