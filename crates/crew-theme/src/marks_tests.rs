use super::Marks;

#[test]
fn marks_keep_their_order_cost_nothing_empty_and_stop_at_the_cap() {
    assert!(Marks::default().is_empty());
    assert_eq!(std::mem::size_of::<Marks>(), 4 * Marks::CAP);
    let m = Marks::of(['\u{94d}', '\u{200d}']);
    assert!(!m.is_empty());
    assert_eq!(m.chars().collect::<String>(), "\u{94d}\u{200d}");
    let mut full = Marks::of("\u{301}\u{302}\u{303}\u{304}".chars());
    assert!(!full.push('\u{305}'), "a fifth mark is dropped");
    assert_eq!(full.chars().count(), Marks::CAP);
    let mut m = Marks::default();
    assert!(!m.push('\0'), "NUL is the empty slot, never a mark");
    assert!(m.is_empty());
}
