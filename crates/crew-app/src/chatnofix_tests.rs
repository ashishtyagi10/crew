//! A fix request that changed nothing says so; an honest answer, a
//! question, an edit or a receipt keeps it quiet.
use super::{asks_a_fix, claims_a_fix, note, NOTE};
use crate::chatlayout::Message;

const RECEIPT: &str = "task #1: 1 file changed (+1 \u{2212}1): ~ lib.rs\n```diff\n-a\n+b\n```";

fn m(sender: &str, text: &str) -> Message {
    Message {
        sender: sender.into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
    }
}

fn turn(ask: &str, answer: &str) -> Vec<Message> {
    vec![
        m("user", "earlier"),
        m("claude \u{2192} user", "Fixed it earlier."),
        m("user", ask),
        m("agent smith", "routing: reply"),
        m("claude", "[tool] Read src/lib.rs \u{2713}"),
        m("claude \u{2192} user", answer),
    ]
}

const ASK: &str = "cargo test -p crew-theme fails, fix it";

#[test]
fn a_fix_that_edited_nothing_and_claims_it_is_named_a_diagnosis() {
    let claimed = "The channels were swapped in `rgb()`. Fixed \u{2014} the test passes now.";
    assert_eq!(note(&turn(ASK, claimed), false), Some(NOTE));
}

#[test]
fn an_edit_or_a_receipt_or_an_ending_keeps_it_quiet() {
    let claimed = "Fixed: the channels were swapped.";
    assert_eq!(
        note(&turn(ASK, claimed), true),
        None,
        "chatproof heard an edit"
    );
    let mut with_receipt = turn(ASK, claimed);
    with_receipt.push(m("agent smith", RECEIPT));
    assert_eq!(
        note(&with_receipt, false),
        None,
        "the broker saw files change"
    );
    let mut stopped = turn(ASK, claimed);
    stopped.push(m(
        "agent smith",
        "\u{2717} task #4 stopped \u{2014} no file changed",
    ));
    assert_eq!(note(&stopped, false), None, "the ending already says it");
}

#[test]
fn a_question_or_an_honest_answer_gets_no_note() {
    let claimed = "Fixed \u{2014} it passes now.";
    assert_eq!(
        note(&turn("why does the theme test fail?", claimed), false),
        None
    );
    let honest = "The cause is a swapped channel in `rgb()`. Swapping them back would fix it.";
    assert_eq!(note(&turn(ASK, honest), false), None);
    // The earlier turn's "Fixed" is not this turn's answer.
    assert_eq!(
        note(&turn(ASK, "I could not reproduce the failure."), false),
        None
    );
}

#[test]
fn the_ask_is_a_fix_by_its_words() {
    assert!(asks_a_fix("Fix the failing test"));
    assert!(asks_a_fix("can you repair the build"));
    assert!(asks_a_fix("fixing the crash please"));
    assert!(!asks_a_fix("add a prefix to the log line"));
    assert!(!asks_a_fix("what does this function do"));
}

#[test]
fn a_hedged_claim_is_not_a_claim() {
    assert!(claims_a_fix("Fixed the off-by-one in `wrap()`."));
    assert!(claims_a_fix(
        "I\u{2019}ve updated `rgb()`; the test passes now."
    ));
    assert!(claims_a_fix("This should now work."));
    assert!(!claims_a_fix("It isn't fixed yet."));
    assert!(!claims_a_fix("This is already fixed on main."));
    assert!(!claims_a_fix("Once you swap them, it is fixed."));
    assert!(!claims_a_fix(
        "I changed nothing: the bug is in the caller."
    ));
    assert!(!claims_a_fix("Is it fixed for you?"));
    assert!(!claims_a_fix(
        "Here is the cause: the channels are swapped."
    ));
}

/// End to end through the pane: the note lands under the answer when the
/// task ends, and a turn that edited lets `chatproof` speak instead.
#[test]
fn the_pane_files_the_note_when_the_task_ends() {
    let mut p = crate::chat::tests::pane();
    p.messages.push(m("user", ASK));
    p.absorb_task(9, true);
    p.absorb_message(
        "claude \u{2192} user".into(),
        "Fixed: the channels were swapped.".into(),
        String::new(),
        String::new(),
    );
    p.absorb_task(9, false);
    let last = p.messages.last().expect("a note");
    assert_eq!(
        (last.sender.as_str(), last.text.as_str()),
        ("agent smith", NOTE)
    );
}
