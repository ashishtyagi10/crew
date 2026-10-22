use std::sync::mpsc;

use winit::keyboard::{Key, NamedKey};

use super::Line;
use crate::inbox::api::Mail;
use crate::inbox::wire::Thread;
use crate::inbox::worker::Worker;
use crate::inbox::{InboxPane, Reading, Stage};

/// A pane reading a thread whose newest mail, sent Tuesday Oct 20 2026,
/// asks for the Q4 numbers "by Friday".
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
            text: "Hi,\n\nCan you send me the Q4 numbers by Friday?\n\nAnn".into(),
            ..Mail::default()
        }]),
        ..Reading::new(thread)
    });
    p
}

fn press(p: &mut InboxPane, key: Key) {
    p.press(&key, 90, 30);
}

fn named(n: NamedKey) -> Key {
    Key::Named(n)
}

fn typed(p: &mut InboxPane, s: &str) {
    for ch in s.chars() {
        press(p, Key::Character(ch.to_string().into()));
    }
}

fn shown(p: &InboxPane) -> String {
    p.cells(90, 30).iter().map(|c| c.c).collect()
}

/// `t` opens the card and adds nothing yet; the due date is the Friday after
/// the mail was sent, whenever `t` is pressed, and says why.
#[test]
fn t_opens_the_card_filled_in_and_adds_nothing_yet() {
    let _g = crate::todopane::store::test_guard(Vec::new());
    let mut p = reading();
    typed(&mut p, "t");
    assert!(
        crate::todopane::store::snapshot().is_empty(),
        "nothing lands before Enter"
    );
    let c = p.todo.as_ref().expect("the card");
    assert_eq!(c.text(Line::Title), "Reply to Ann Lee: Q4 numbers");
    assert_eq!(
        c.text(Line::Due),
        "2026-10-23",
        "the Friday after Tue Oct 20"
    );
    assert!(
        c.why[3].contains("\u{201c}by friday\u{201d} in Ann Lee"),
        "{}",
        c.why[3]
    );
    assert!(c.why[3].ends_with("mail of Oct 20"), "{}", c.why[3]);
    let text = shown(&p);
    for word in ["todo", "title", "project", "assignee", "due", "2026-10-23"] {
        assert!(text.contains(word), "{word}: {text}");
    }
}

/// Enter adds the row as the card says it: the lines you changed, tags as
/// `/todo` keeps them.
#[test]
fn enter_adds_the_row_as_the_card_says_it() {
    let _g = crate::todopane::store::test_guard(Vec::new());
    let mut p = reading();
    typed(&mut p, "t");
    press(&mut p, named(NamedKey::ArrowDown));
    typed(&mut p, "@crew");
    press(&mut p, named(NamedKey::ArrowDown));
    typed(&mut p, "bo park");
    press(&mut p, named(NamedKey::Enter));
    let items = crate::todopane::store::snapshot();
    assert_eq!(items.len(), 1);
    let it = &items[0];
    assert_eq!(it.title, "Reply to Ann Lee: Q4 numbers");
    assert_eq!(
        (it.project.as_deref(), it.assignee.as_deref()),
        (Some("crew"), Some("bo-park"))
    );
    assert!(it.due_ms.is_some() && !it.due_has_time);
    assert!(p.todo.is_none(), "the card goes once the row is in");
    assert!(
        p.note
            .starts_with("added to /todo: Reply to Ann Lee: Q4 numbers @crew #bo-park"),
        "{}",
        p.note
    );
}

/// A due line that is not a date `/todo` reads is refused, the card kept;
/// a typed one reads as of now, and the card says what it reads as.
#[test]
fn a_due_line_must_read_as_a_date() {
    let _g = crate::todopane::store::test_guard(Vec::new());
    let mut p = reading();
    typed(&mut p, "t");
    p.todo.as_mut().unwrap().on = Line::Due;
    for _ in 0..12 {
        press(&mut p, named(NamedKey::Backspace));
    }
    typed(&mut p, "someday");
    press(&mut p, named(NamedKey::Enter));
    assert!(p.note.contains("is not a date"), "{}", p.note);
    assert!(p.todo.is_some() && crate::todopane::store::snapshot().is_empty());
    assert!(shown(&p).contains("not a date /todo reads"));
    for _ in 0..7 {
        press(&mut p, named(NamedKey::Backspace));
    }
    typed(&mut p, "tomorrow");
    assert!(shown(&p).contains("\u{2192} "), "what it reads as");
    press(&mut p, named(NamedKey::Enter));
    assert_eq!(crate::todopane::store::snapshot().len(), 1);
}

/// Esc lets an unchanged card go at once, and a changed one only when asked
/// twice, as the reply box does.
#[test]
fn esc_lets_an_unchanged_card_go_and_asks_twice_for_a_changed_one() {
    let _g = crate::todopane::store::test_guard(Vec::new());
    let mut p = reading();
    typed(&mut p, "t");
    press(&mut p, named(NamedKey::Escape));
    assert!(
        p.todo.is_none() && p.reading.is_some(),
        "the card, not the thread"
    );
    typed(&mut p, "t");
    typed(&mut p, "!");
    press(&mut p, named(NamedKey::Escape));
    assert!(p.todo.is_some() && p.note == "esc again discards it");
    press(&mut p, named(NamedKey::Escape));
    assert!(p.todo.is_none() && crate::todopane::store::snapshot().is_empty());
}

/// With the card open another thread waits, a paste lands on its line, and
/// a click on a line puts the caret there.
#[test]
fn the_card_holds_the_thread_takes_a_paste_and_a_click() {
    let mut p = reading();
    p.rows.push(Thread {
        id: "t2".into(),
        ..Thread::default()
    });
    typed(&mut p, "t");
    assert!(!p.open_at(1));
    assert!(p.note.starts_with("a todo is open"), "{}", p.note);
    let b = p.cards(90, 30).reply.expect("the card's place");
    assert!(p.todo_click(90, 30, b.row + 2, b.col + 20));
    assert_eq!(p.todo.as_ref().unwrap().on, Line::Project);
    p.paste("design");
    assert_eq!(p.todo.as_ref().unwrap().text(Line::Project), "design");
}
