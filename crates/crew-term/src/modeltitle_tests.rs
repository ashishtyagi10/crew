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
    // alacritty empties its own stack too, so the title above proves nothing
    // about crew's count: THIS does (nothing left for a dead program to unwind).
    c.feed(b"\x1b[22;0t\x1bc");
    assert!(!c.title_unwind());
}

#[test]
fn the_stack_reaches_the_newest_ten() {
    let mut c = core();
    for i in 1..=12 {
        set(&mut c, &format!("t{i}"));
        c.feed(b"\x1b[22;0t");
    }
    set(&mut c, "now");
    for _ in 0..10 {
        c.feed(b"\x1b[23;0t");
    }
    assert_eq!(c.title(), "t3", "t1 and t2 are out of reach");
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

/// A shell's precmd sets its title BEFORE the `A` mark but after `D`: the
/// unwind must happen at `D`, or it pops a stale title over the fresh one.
#[test]
fn the_shells_fresh_title_survives_the_unwind() {
    let mut c = core();
    set(&mut c, "shell");
    c.feed(b"\x1b[22;0t");
    set(&mut c, "vim foo");
    // vim is killed: no pop. The shell reports done, precmd titles, prompts.
    c.feed(b"\x1b]133;D;137\x07\x1b]0;shell2\x07\x1b]133;A\x07$ ");
    assert_eq!(c.title(), "shell2");
    assert!(!c.title_unwind(), "the D mark already unwound it");
}

/// The same, with the whole sequence arriving a byte at a time.
#[test]
fn the_unwind_at_done_holds_when_split_across_reads() {
    let mut c = core();
    set(&mut c, "shell");
    c.feed(b"\x1b[22;0t");
    set(&mut c, "vim foo");
    for &b in b"\x1b]133;D;1\x07\x1b]0;shell2\x07\x1b]133;A\x07" {
        c.feed(&[b]);
    }
    assert_eq!(c.title(), "shell2");
    assert!(!c.title_unwind(), "unwound at D, split or not");
}

/// Without OSC 133 there is no safe moment (the shell's own title may
/// already be there), so nothing is unwound and nothing is clobbered.
#[test]
fn a_shell_without_marks_keeps_its_title() {
    let mut c = core();
    set(&mut c, "shell");
    c.feed(b"\x1b[22;0t");
    set(&mut c, "vim foo");
    set(&mut c, "shell2");
    assert_eq!(c.title(), "shell2");
}
