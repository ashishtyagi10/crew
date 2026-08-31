use super::*;

#[test]
fn history_caps_and_keeps_newest() {
    let mut h = History::new(3);
    for v in [1, 2, 3, 4, 5] {
        h.push(v);
    }
    // capacity 3 keeps the three newest, oldest first
    assert_eq!(h.tail(10), vec![3, 4, 5]);
}

#[test]
fn peak_scans_the_visible_window_only() {
    let mut h = History::new(8);
    for v in [90, 10, 20, 30] {
        h.push(v);
    }
    assert_eq!(h.peak(3), 30, "the 90 is outside the 3-sample window");
    assert_eq!(h.peak(10), 90);
    assert_eq!(History::new(4).peak(4), 0, "empty history peaks at 0");
}

#[test]
fn tail_returns_at_most_width() {
    let mut h = History::new(10);
    for v in [10, 20, 30, 40] {
        h.push(v);
    }
    assert_eq!(h.tail(2), vec![30, 40]);
}
