use super::level_of;
use crate::md::{render, render_chat};

/// Every level is bold, and the LEVEL survives to the laid-out line — on
/// main it did too, but only h1 was ever checked.
#[test]
fn every_level_is_bold_and_keeps_its_level() {
    for (src, level) in [("# a", 1), ("## b", 2), ("### c", 3), ("###### f", 6)] {
        let lines = render(src, 40);
        assert_eq!(level_of(&lines[0]), level, "{src}");
        assert!(lines[0].spans.iter().all(|s| s.style.bold), "{src}");
    }
}

/// A wrapped heading carries its level on every row; a quoted one reads
/// past the bar; prose and a rule are level 0.
#[test]
fn level_of_reads_past_wrapping_and_a_quote_bar() {
    let wrapped = render_chat("# one two three", 8);
    assert_eq!(wrapped.len(), 2, "{wrapped:?}");
    assert!(wrapped.iter().all(|l| level_of(l) == 1));
    let quoted = render_chat("> ## inside", 40);
    assert!(quoted[0].spans[0].style.marker);
    assert_eq!(level_of(&quoted[0]), 2);
    assert_eq!(level_of(&render_chat("body", 40)[0]), 0);
    assert_eq!(level_of(&render_chat("---", 40)[0]), 0);
}
