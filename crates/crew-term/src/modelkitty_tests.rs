//! The kitty keyboard protocol's stacks as a program drives them: the query,
//! push and pop, one stack per screen, the depth cap and the prompt's pop.
use crate::model::{GridSize, TermCore};

fn core() -> TermCore {
    TermCore::new(GridSize { cols: 30, rows: 4 })
}

/// The flags a program learns from `CSI ? u`.
fn ask(c: &mut TermCore) -> Option<String> {
    c.feed(b"\x1b[?u");
    c.take_replies()
}

fn on(c: &TermCore) -> bool {
    c.input_modes().kitty_keys
}

#[test]
fn the_query_answers_what_was_pushed_and_popped() {
    let mut c = core();
    assert_eq!(
        ask(&mut c).as_deref(),
        Some("\x1b[?0u"),
        "a terminal that speaks it"
    );
    assert!(!on(&c));
    c.feed(b"\x1b[>1u");
    assert_eq!(ask(&mut c).as_deref(), Some("\x1b[?1u"));
    assert!(on(&c), "the encoder's switch");
    c.feed(b"\x1b[<u");
    assert_eq!(ask(&mut c).as_deref(), Some("\x1b[?0u"));
    assert!(!on(&c));
}

/// Only "disambiguate" is honoured, so a program asking for all five flags
/// learns it got that one, and one asking for another flag alone, none.
#[test]
fn the_answer_names_only_the_flag_crew_honours() {
    let mut c = core();
    c.feed(b"\x1b[>31u");
    assert_eq!(ask(&mut c).as_deref(), Some("\x1b[?1u"));
    assert!(on(&c));
    c.feed(b"\x1b[>8u");
    assert_eq!(ask(&mut c).as_deref(), Some("\x1b[?0u"));
    assert!(!on(&c));
}

#[test]
fn each_screen_keeps_its_own_stack() {
    let mut c = core();
    c.feed(b"\x1b[>1u\x1b[?1049h");
    assert!(!on(&c), "the alternate screen starts empty");
    c.feed(b"\x1b[>1u\x1b[>1u\x1b[<u");
    assert!(on(&c));
    c.feed(b"\x1b[<u\x1b[?1049l");
    assert!(on(&c), "the primary's push is still there");
}

/// alacritty's push at 4096 deep removes from the (empty) title stack and
/// panics: `cat` of such a file must not take the app down. Past the cap a
/// push is dropped, and the stack below it is whole.
#[test]
fn a_flood_of_pushes_is_capped_not_a_crash() {
    let mut c = core();
    c.feed(&b"\x1b[>1u".repeat(4100));
    assert!(on(&c));
    c.feed(&b"\x1b[<u".repeat(255));
    assert!(on(&c), "255 pops leave one of the 256 kept");
    c.feed(b"\x1b[<u");
    assert!(
        !on(&c),
        "and the next pops it: the dropped ones never landed"
    );
}

/// nvim pushes on the alternate screen and is killed: the prompt's cut
/// empties that stack before leaving it, so the next `less` doesn't inherit
/// it, and the primary's too.
#[test]
fn a_prompt_mark_empties_both_stacks() {
    let mut c = core();
    c.feed(b"\x1b[>1u\x1b[?1049h\x1b[>1u");
    c.feed(b"\x1b]133;A\x07$ ");
    assert!(!on(&c), "the primary's flags are gone");
    assert!(!c.command_marks(), "a prompt mark is not a command mark");
    c.feed(b"\x1b[?1049h");
    assert!(!on(&c), "the alternate screen's are gone too");
}

/// fish 4 asks for the protocol once its prompt is drawn: a push AFTER the
/// mark, even in the same read, is the shell's own and stays.
#[test]
fn a_push_after_the_mark_is_kept() {
    let mut c = core();
    c.feed(b"\x1b[>1u");
    c.feed(b"\x1b]133;A\x07$ \x1b[>5u");
    assert!(on(&c));
    assert_eq!(ask(&mut c).as_deref(), Some("\x1b[?1u"));
}

/// A shell redraws its prompt (a resize, a repaint), mark and all, with no
/// command between: the protocol it turned on after the first mark stays.
/// After a command (`D`), the next prompt empties the stacks again.
#[test]
fn only_the_first_prompt_after_a_command_pops() {
    let mut c = core();
    c.feed(b"\x1b]133;A\x07$ \x1b[=5u");
    assert!(on(&c), "the shell's own, set after its prompt");
    c.feed(b"\r\x1b]133;A\x07$ ");
    assert!(on(&c), "a redrawn prompt keeps it");
    c.feed(b"\x1b[=0u\x1b]133;C\x07\x1b[>1u");
    c.feed(b"\x1b]133;D;137\x07\x1b]133;A\x07$ ");
    assert!(!on(&c), "the killed program's push went at the next prompt");
    assert!(
        c.command_marks(),
        "so the leftover reset leaves it to the mark"
    );
}

/// The parser takes a push in other spellings, and ignores a "pop" with a
/// digit before its `<`: neither gets past the cap.
#[test]
fn the_cap_holds_against_other_spellings() {
    let long = format!("\x1b[>1{}u", ";1".repeat(20));
    for push in [&b"\x1b[>1:2u"[..], long.as_bytes(), b"\x1b[>1u\x1b[0<u"] {
        let mut c = core();
        c.feed(&push.repeat(4100));
        c.feed(b"ok");
    }
}
