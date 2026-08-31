use super::*;

#[test]
fn take_prev_empties_the_slots() {
    let mut s = SceneSlots::default();
    s.set(vec![1, 2], Vec::new());
    let (sigs, bufs) = s.take_prev();
    assert_eq!(sigs, vec![1, 2]);
    assert!(bufs.is_empty());
    assert!(s.bufs().is_empty());
    assert!(s.take_prev().0.is_empty(), "second take sees empty state");
}
