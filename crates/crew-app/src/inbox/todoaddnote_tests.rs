//! What the note says of the row Enter filed: a due already passed says so.
use std::sync::mpsc;

use winit::keyboard::{Key, NamedKey};

use super::super::Line;
use crate::inbox::api::Mail;
use crate::inbox::wire::Thread;
use crate::inbox::worker::Worker;
use crate::inbox::{InboxPane, Reading, Stage};

fn reading() -> InboxPane {
    let ((tx, _), (_, rx)) = (mpsc::channel(), mpsc::channel());
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    let thread = Thread {
        id: "t1".into(),
        subject: "Q4 numbers".into(),
        ..Thread::default()
    };
    p.rows = vec![thread.clone()];
    p.reading = Some(Reading {
        mails: Some(vec![Mail {
            from: "Ann Lee <ann@x.org>".into(),
            date: "Tue, 20 Oct 2026 09:02:00 -0400".into(),
            text: "Hi,\n\nThe Q4 numbers were due by Friday.\n\nAnn".into(),
            ..Mail::default()
        }]),
        ..Reading::new(thread)
    });
    p
}

fn typed(p: &mut InboxPane, s: &str) {
    for ch in s.chars() {
        p.press(&Key::Character(ch.to_string().into()), 90, 30);
    }
}

/// A due long passed, filed as the card says it: the note gives its year
/// and says it has passed, where it read "(due Mon Jan 6)" as if ahead.
#[test]
fn a_passed_due_is_said_to_have_passed() {
    let _g = crate::todopane::store::test_guard(Vec::new());
    let mut p = reading();
    typed(&mut p, "t");
    p.todo.as_mut().unwrap().on = Line::Due;
    for _ in 0..24 {
        p.press(&Key::Named(NamedKey::Backspace), 90, 30);
    }
    typed(&mut p, "2020-01-06");
    p.press(&Key::Named(NamedKey::Enter), 90, 30);
    assert!(
        p.note.ends_with("(due Mon Jan 6 2020, passed)"),
        "{}",
        p.note
    );
    assert!(crate::todopane::store::snapshot()[0].notified);
}
