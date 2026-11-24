use super::*;

/// The same text answers from the memo; an edit, or another width, renders again — and what
/// comes back is always what a fresh render gives.
#[test]
fn a_render_is_kept_per_text_and_width() {
    let a = "# One\n\nbody\n";
    let first = render(a, 40);
    assert!(Rc::ptr_eq(&first, &render(a, 40)), "kept");
    assert!(!Rc::ptr_eq(&first, &render(a, 30)), "another width renders");
    let b = "# One\n\nbody, edited\n";
    let edited = render(b, 30);
    assert_eq!(*edited, crate::md::render(b, 30), "an edit renders afresh");
    assert_ne!(*edited, *render(a, 30));
}

/// The blocks follow the text: an added paragraph is a block the next read finds.
#[test]
fn blocks_follow_the_text() {
    let a = "one\n\ntwo\n";
    assert!(Rc::ptr_eq(&walk(a), &walk(a)));
    assert_eq!(walk(a).leaves.len(), 2);
    assert_eq!(walk("one\n\ntwo\n\nthree\n").leaves.len(), 3);
}
