//! The title stack as vim, tmux and less drive it: push on entry, pop on exit.
use crate::model::{GridSize, TermCore};

fn core() -> TermCore {
    TermCore::new(GridSize { cols: 30, rows: 4 })
}

fn set(c: &mut TermCore, t: &str) {
    c.feed(format!("\x1b]0;{t}\x07").as_bytes());
}

#[test]
fn a_pop_brings_back_what_a_push_saved() {
    let mut c = core();
    set(&mut c, "shell");
    c.feed(b"\x1b[22;0t");
    set(&mut c, "vim");
    assert_eq!(c.title(), "vim");
    c.feed(b"\x1b[23;0t");
    assert_eq!(c.title(), "shell");
}

#[test]
fn title_only_and_bare_forms_count_too() {
    let mut c = core();
    set(&mut c, "shell");
    c.feed(b"\x1b[22;2t");
    set(&mut c, "a");
    c.feed(b"\x1b[22t");
    set(&mut c, "b");
    c.feed(b"\x1b[23t");
    assert_eq!(c.title(), "a");
    c.feed(b"\x1b[23;2t");
    assert_eq!(c.title(), "shell");
}

#[test]
fn pushes_nest() {
    let mut c = core();
    set(&mut c, "shell");
    c.feed(b"\x1b[22;0t");
    set(&mut c, "tmux");
    c.feed(b"\x1b[22;0t");
    set(&mut c, "vim");
    c.feed(b"\x1b[23;0t");
    assert_eq!(c.title(), "tmux");
    c.feed(b"\x1b[23;0t");
    assert_eq!(c.title(), "shell");
}

#[test]
fn a_pop_with_nothing_pushed_changes_nothing() {
    let mut c = core();
    set(&mut c, "shell");
    c.feed(b"\x1b[23;0t\x1b[23t");
    assert_eq!(c.title(), "shell");
    assert_eq!(c.take_replies(), None);
}

#[test]
fn the_icon_alone_is_left_out() {
    let mut c = core();
    set(&mut c, "shell");
    // vim pushes the icon name too: that one must not take a title's place.
    c.feed(b"\x1b[22;1t");
    set(&mut c, "vim");
    c.feed(b"\x1b[23;1t");
    assert_eq!(c.title(), "vim", "23;1 popped nothing");
    c.feed(b"\x1b[23;0t");
    assert_eq!(c.title(), "vim", "22;1 pushed nothing, so 23;0 has none");
}

#[test]
fn ris_forgets_the_stack() {
    let mut c = core();
    set(&mut c, "shell");
    c.feed(b"\x1b[22;0t");
    set(&mut c, "vim");
    c.feed(b"\x1bc");
    set(&mut c, "after");
    c.feed(b"\x1b[23;0t");
    assert_eq!(c.title(), "after");
    // crew's own count went too: nothing left for a dead program to unwind.
    c.feed(b"\x1b[22;0t\x1bc");
    assert!(!c.title_unwind());
}

#[test]
fn the_stack_holds_ten_and_forgets_the_oldest() {
    let mut c = core();
    for i in 1..=12 {
        set(&mut c, &format!("t{i}"));
        c.feed(b"\x1b[22;0t");
    }
    set(&mut c, "now");
    for _ in 0..10 {
        c.feed(b"\x1b[23;0t");
    }
    assert_eq!(c.title(), "t3", "t1 and t2 fell off the bottom");
    c.feed(b"\x1b[23;0t");
    assert_eq!(c.title(), "t3", "an eleventh pop has nothing");
}

#[test]
fn a_sequence_split_across_reads_still_counts() {
    let mut c = core();
    set(&mut c, "shell");
    for &b in b"\x1b[22;0t" {
        c.feed(&[b]);
    }
    set(&mut c, "vim");
    for &b in b"\x1b[23;1t" {
        c.feed(&[b]);
    }
    assert_eq!(c.title(), "vim", "split 23;1 is still icon-only");
    for &b in b"\x1b[23;0t" {
        c.feed(&[b]);
    }
    assert_eq!(c.title(), "shell");
}

#[test]
fn a_killed_programs_pushes_unwind_to_the_title_before_it() {
    let mut c = core();
    set(&mut c, "shell");
    c.feed(b"\x1b[22;0t");
    set(&mut c, "tmux");
    c.feed(b"\x1b[22;0t");
    set(&mut c, "vim");
    assert!(c.title_unwind());
    assert_eq!(c.title(), "shell", "the bottom-most, saved before it all");
    assert!(!c.title_unwind(), "nothing left to unwind");
    c.feed(b"\x1b[23;0t");
    assert_eq!(c.title(), "shell");
}

#[test]
fn an_unwind_with_nothing_pushed_leaves_the_title() {
    let mut c = core();
    set(&mut c, "shell");
    assert!(!c.title_unwind());
    assert_eq!(c.title(), "shell");
}
