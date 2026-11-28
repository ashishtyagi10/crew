//! Which message started which task, and what a rewind leaves alone.
use super::fix::*;
use super::*;

fn typed(p: &mut ChatPane, ts: &str, text: &str) {
    say(p, "user", ts, text);
}

fn bound_ts(p: &ChatPane) -> Vec<String> {
    p.rewind.picks().iter().map(|m| m.ts.clone()).collect()
}

#[test]
fn a_refused_send_never_lends_its_message_to_a_later_identical_one() {
    let (mut p, file) = pane();
    typed(&mut p, "1", "x");
    p.send_now("x".into()); // refused at capacity: no task starts
    typed(&mut p, "2", "x");
    p.send_now("x".into());
    let lines = sent(&file, 2);
    assert!(lines[0].contains(r#""seq":1"#) && lines[1].contains(r#""seq":2"#));
    started(&mut p, 5, 2); // the broker answers the second send only
    assert_eq!(bound_ts(&p), ["2"], "the second message owns the task");
    assert!(
        p.rewind.marks.iter().all(|m| m.ts != "1"),
        "the refused one was dropped"
    );
}

#[test]
fn a_message_pulled_back_from_the_queue_is_forgotten() {
    let (mut p, _) = pane();
    typed(&mut p, "1", "later");
    p.queued.push_back("later".into());
    assert!(crate::chatqueue::take_back(&mut p));
    assert!(p.rewind.marks.is_empty(), "it is in the composer again");
    p.input.clear();
    typed(&mut p, "2", "later");
    let n = p.rewind_stamp("later");
    started(&mut p, 3, n);
    assert_eq!(bound_ts(&p), ["2"]);
}

#[test]
fn a_cancelled_queue_and_a_steered_message_leave_no_mark_to_bind() {
    let (mut p, _) = pane();
    typed(&mut p, "1", "one");
    typed(&mut p, "2", "two");
    p.queued.extend(["one".to_string(), "two".to_string()]);
    p.absorb_steered("one".into()); // the running task took it
    assert_eq!(p.rewind.marks.len(), 1);
    assert!(p.drop_queue().is_some());
    assert!(p.rewind.marks.is_empty(), "{:?}", p.rewind.marks);
    // A later identical send binds to its own mark.
    typed(&mut p, "3", "two");
    let n = p.rewind_stamp("two");
    started(&mut p, 8, n);
    assert_eq!(bound_ts(&p), ["3"]);
}

#[test]
fn sends_are_matched_in_order_and_an_old_broker_binds_nothing() {
    let (mut p, _) = pane();
    typed(&mut p, "1", "same");
    typed(&mut p, "2", "same"); // both waiting in the queue
    let first = p.rewind_stamp("same");
    let second = p.rewind_stamp("same");
    assert_eq!((first, second), (1, 2), "first in, first out");
    started(&mut p, 1, 0); // a broker that does not echo numbers
    assert!(p.rewind.picks().is_empty());
    started(&mut p, 1, first);
    started(&mut p, 2, second);
    assert_eq!(bound_ts(&p), ["2", "1"]);
}

#[test]
fn a_send_with_no_message_behind_it_takes_no_number_from_one() {
    let (mut p, _) = pane();
    typed(&mut p, "1", "queued words");
    assert_eq!(p.rewind_stamp("/init"), 0, "the app's own command");
    assert_eq!(p.rewind_stamp("queued words and a file body"), 1);
}

#[test]
fn queued_messages_stop_a_rewind_and_survive_it() {
    let (mut p, file) = history();
    p.queued.push_back("waiting".into());
    key(&mut p, ChatInput::Rewind);
    assert!(p.rewind.sel.is_none());
    let note = &p.messages.last().unwrap().text;
    assert_eq!(note, "rewind: send or clear the queued messages first");
    // Queued after the list opened: Enter on a row is refused.
    p.queued.clear();
    key(&mut p, ChatInput::Rewind);
    p.queued.push_back("waiting".into());
    key(&mut p, ChatInput::Enter);
    assert!(p.rewind.want.is_none() && sent(&file, 1).is_empty());
    // Queued after the ask went out: Enter on the confirmation is refused too.
    p.queued.clear();
    key(&mut p, ChatInput::Rewind);
    key(&mut p, ChatInput::Enter);
    p.rewind_hears(&listed(2, &["a.rs"], &[], false));
    p.queued.push_back("waiting".into());
    key(&mut p, ChatInput::Enter);
    assert_eq!(sent(&file, 1).len(), 1, "no yes was sent");
    assert!(p.rewind.ask.is_none() && p.rewind.want.is_none());
    // And a rewind that does go through leaves the queue alone.
    p.queued.clear();
    key(&mut p, ChatInput::Rewind);
    key(&mut p, ChatInput::Enter);
    p.rewind_hears(&answer(2, 0, false, false, ""));
    p.queued.push_back("late".into());
    p.rewind_hears(&answer(2, 0, false, true, ""));
    assert_eq!(p.queued, ["late"]);
}
