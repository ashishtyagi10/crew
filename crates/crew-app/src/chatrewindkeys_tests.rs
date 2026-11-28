use super::fix::*;
use super::*;

#[test]
fn escape_closes_the_list_and_keeps_on_the_confirmation() {
    let (mut p, file) = history();
    key(&mut p, ChatInput::Rewind);
    key(&mut p, ChatInput::Close);
    assert!(p.rewind.sel.is_none() && p.popup(100).is_none());
    key(&mut p, ChatInput::Rewind);
    key(&mut p, ChatInput::Enter);
    p.rewind_hears(&answer(2, 1, false, false, ""));
    key(&mut p, ChatInput::Close);
    assert!(p.rewind.ask.is_none() && p.popup(100).is_none());
    let n = p.messages.len();
    p.rewind_hears(&answer(2, 1, false, true, "")); // a late answer finds nothing wanted
    assert_eq!(p.messages.len(), n);
    assert_eq!(sent(&file, 1).len(), 1, "only the ask went out");
}

#[test]
fn a_broker_refusal_is_said_and_nothing_is_cut() {
    let (mut p, _) = history();
    let before = p.messages.len();
    key(&mut p, ChatInput::Rewind);
    key(&mut p, ChatInput::Enter);
    p.rewind_hears(&answer(2, 0, false, false, "stop it first (/stop)"));
    assert_eq!(p.messages.len(), before + 1);
    assert!(p
        .messages
        .last()
        .unwrap()
        .text
        .contains("rewind: stop it first"));
    assert!(p.input.is_empty());
}

#[test]
fn text_in_the_composer_keeps_the_chord_from_opening_and_says_why() {
    let (mut p, _) = history();
    p.input = "half typed".into();
    key(&mut p, ChatInput::Rewind);
    assert!(p.rewind.sel.is_none());
    assert_eq!(p.input, "half typed");
    assert_eq!(
        p.messages.last().unwrap().text,
        "rewind needs an empty composer"
    );
}

#[test]
fn a_task_label_made_from_an_expanded_message_still_binds() {
    let (mut p, _) = pane();
    say(&mut p, "user", "10", "look at @a.rs");
    // `send_now` gets the expanded text, the file bodies appended.
    let n = p.rewind_stamp("look at @a.rs\n\n--- a.rs ---\nfn main");
    started(&mut p, 4, n);
    assert_eq!(p.rewind.picks()[0].task, Some(4));
}

#[test]
fn a_broker_restart_forgets_the_list() {
    let (mut p, _) = history();
    p.reset_broker_state();
    assert!(p.rewind.picks().is_empty());
}
