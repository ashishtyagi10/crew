use super::*;

#[test]
fn a_failure_says_how_to_fix_it_and_a_success_says_nothing() {
    let t = text(Some(101)).expect("a hint");
    assert!(t.starts_with("failed (exit 101) \u{2014} "), "{t}");
    assert!(t.ends_with("or click the mark: fix with smith"), "{t}");
    assert!(
        t.contains(&*crate::chordglyph::prose(crate::fixwithsmith::KEY)),
        "{t}"
    );
    assert_eq!(text(Some(0)), None);
    assert_eq!(text(None), None, "a silent shell is not a failure");
}
