//! What a resumed session is sent, cut from the frame the relay built.
use super::{news, seen_with, Seen};

const STANDING: &str = "You are \"claude\", a CLI agent.\n\nTASK:\n(the message from \"user\")\n\n\
     TOOLS: sys:todo\n\nHOW TO REPLY: end with `@done`.\n\nMESSAGE FOR YOU FROM \"user\":\n\
     PROJECT probe\ntop level: README.md, lib.rs";

fn first() -> String {
    format!("{STANDING}\n\nTASK:\nwhat does lib.rs define?")
}

fn follow_up(ask: &str) -> String {
    format!(
        "{STANDING}\n\nTASK:\nEarlier in this conversation:\n\nyou asked: what does lib.rs \
         define?\ncrew answered: `add`.\n\nNow:\n{ask}"
    )
}

#[test]
fn a_follow_up_is_sent_its_message_alone() {
    let seen = seen_with(Seen::new(), &first());
    assert_eq!(
        news(&follow_up("its parameter types?"), &seen),
        "TASK:\nits parameter types?"
    );
}

#[test]
fn a_paragraph_that_changed_since_is_sent_again() {
    let seen = seen_with(Seen::new(), &first());
    let moved = follow_up("and now?").replace("README.md, lib.rs", "README.md, lib.rs, main.rs");
    let got = news(&moved, &seen);
    assert!(got.starts_with("MESSAGE FOR YOU FROM"), "{got}");
    assert!(got.contains("main.rs"), "{got}");
    assert!(got.ends_with("TASK:\nand now?"), "{got}");
    assert!(!got.contains("HOW TO REPLY"), "{got}");
}

#[test]
fn the_message_is_never_cut_even_where_it_repeats_one_sent_before() {
    let ask = "same log:\n\nTOOLS: sys:todo\n\nagain";
    let seen = seen_with(Seen::new(), &first());
    assert_eq!(news(&follow_up(ask), &seen), format!("TASK:\n{ask}"));
}

#[test]
fn without_a_recap_the_last_paragraph_is_the_message() {
    let seen = seen_with(Seen::new(), &first());
    assert_eq!(news(&first(), &seen), "TASK:\nwhat does lib.rs define?");
    let peer = format!("{STANDING}\n\nopencode says: look at main.rs");
    assert_eq!(news(&peer, &seen), "opencode says: look at main.rs");
}

#[test]
fn what_a_session_has_seen_grows_with_every_message() {
    let seen = seen_with(seen_with(Seen::new(), &first()), &follow_up("two?"));
    assert!(seen.contains("Now:\ntwo?"));
    assert!(seen.contains("TASK:\nwhat does lib.rs define?"));
    assert_eq!(news(&follow_up("three?"), &seen), "TASK:\nthree?");
}
