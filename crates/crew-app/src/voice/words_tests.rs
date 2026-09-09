use super::spoken;

#[test]
fn a_reply_written_for_a_screen_is_shortened_for_an_ear() {
    // A transcript is written to be read: code fences, tables, a hundred-line diff. Reading one
    // aloud is unbearable, so the spoken form says what it can and says that it stopped.
    assert_eq!(spoken("one\ntwo   three"), "one two three");
    assert_eq!(
        spoken("here it is:\n```\nfn main() {}\n```\ndone"),
        "here it is: fn main() {} done",
        "the fence markers are not read out"
    );
    let long = "word ".repeat(400);
    let out = spoken(&long);
    assert!(out.chars().count() < 760, "{}", out.chars().count());
    assert!(out.ends_with("the rest is on screen."), "{out}");
}
