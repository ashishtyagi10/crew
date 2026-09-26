//! The errno goes, and the OS sentence reads as part of crew's.
use super::plain;

#[test]
fn the_errno_goes_and_the_sentence_is_lowercased() {
    assert_eq!(
        plain("failed to spawn shell: No such file or directory (os error 2)".into()),
        "failed to spawn shell: no such file or directory"
    );
    assert_eq!(
        plain("blocks: cannot write: Permission denied (os error 13)".into()),
        "blocks: cannot write: permission denied"
    );
}

#[test]
fn an_acronym_and_a_message_without_an_errno_are_left_alone() {
    assert_eq!(
        plain("read: EOF while parsing (os error 5)".into()),
        "read: EOF while parsing"
    );
    assert_eq!(plain("copied 3 lines".into()), "copied 3 lines");
    assert_eq!(plain("Claude said no".into()), "Claude said no");
    assert_eq!(plain("odd (os error x)".into()), "odd (os error x)");
}
