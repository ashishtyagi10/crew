use std::sync::mpsc;

use crate::app::CrewApp;
use crate::chatlayout::Message;
use crate::inbox::handoff::ToSmith;
use crate::inbox::worker::{Ask, Worker};
use crate::inbox::{InboxPane, Stage};
use crate::pane::{Pane, PaneContent};

fn pane(content: PaneContent, label: Option<&str>) -> Pane {
    Pane {
        glide: crate::glide::Glide::default(),
        content,
        grid: crew_term::GridSize { cols: 80, rows: 24 },
        rect: crate::layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: label.map(str::to_string),
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: 0,
        zoom: None,
    }
}

/// An app with the inbox (over a hand-held worker) and smith's pane.
fn app(send: ToSmith) -> (CrewApp, mpsc::Receiver<Ask>) {
    let (tx, asks) = mpsc::channel();
    let (_answer, rx) = mpsc::channel();
    let mut inbox = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    inbox.to_smith = Some(send);
    let mut a = CrewApp::default();
    a.panes
        .push(pane(PaneContent::Inbox(Box::new(inbox)), None));
    a.panes.push(pane(
        PaneContent::Chat(crate::chat::tests::pane()),
        Some("crew"),
    ));
    (a, asks)
}

fn chat(a: &mut CrewApp) -> &mut crate::chat::ChatPane {
    match &mut a.panes[1].content {
        PaneContent::Chat(c) => c,
        _ => unreachable!(),
    }
}

fn inbox(a: &CrewApp) -> &InboxPane {
    match &a.panes[0].content {
        PaneContent::Inbox(p) => p,
        _ => unreachable!(),
    }
}

#[test]
fn a_summary_goes_to_smith_showing_one_line_and_smith_takes_the_focus() {
    let send = ToSmith {
        prompt: "<<<MAIL\nlong thread\nMAIL>>>".into(),
        shown: "\u{2709} what does it want?".into(),
        wants: crate::inbox::handoff::Wants::Read,
        thread: "t1".into(),
    };
    let (mut a, _asks) = app(send);
    assert!(a.poll_inbox());
    let last = chat(&mut a)
        .messages
        .last()
        .map(|m| (m.sender.clone(), m.text.clone()));
    assert_eq!(
        last,
        Some(("user".into(), "\u{2709} what does it want?".into()))
    );
    assert!(chat(&mut a).is_busy(), "the prompt was sent");
    assert_eq!(a.focused, 1);
    assert!(inbox(&a).to_smith.is_none() && inbox(&a).waiting.is_none());
}

#[test]
fn a_draft_waits_for_smiths_settled_answer_and_lets_it_go_with_no_box() {
    let send = ToSmith {
        prompt: "draft it".into(),
        shown: "\u{2709} draft a reply".into(),
        wants: crate::inbox::handoff::Wants::Draft,
        thread: "t1".into(),
    };
    let (mut a, asks) = app(send);
    a.poll_inbox();
    assert_eq!(a.focused, 0, "the inbox keeps the keys");
    assert!(inbox(&a).waiting.is_some());
    let say = |text: &str| Message {
        sender: "agent smith".into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
    };
    // Mid-turn, smith says where it routed: not its answer yet.
    chat(&mut a)
        .messages
        .push(say("routing: reply \u{2192} engineer"));
    assert!(!a.poll_inbox(), "smith is still working");
    assert!(inbox(&a).waiting.is_some());
    let c = chat(&mut a);
    c.awaiting = false;
    c.messages
        .push(say("Here is a reply:\n<<<DRAFT\nSure, Friday.\nDRAFT>>>"));
    c.messages.push(say("\u{2713} checked"));
    assert!(a.poll_inbox());
    assert!(asks.try_recv().is_err(), "nothing saved to Gmail");
    assert!(inbox(&a).note.contains("let go"), "{}", inbox(&a).note);
    assert!(inbox(&a).waiting.is_none());
}
