use std::sync::mpsc;

use super::Rect;
use crate::inbox::api::Mail;
use crate::inbox::wire::Thread;
use crate::inbox::worker::Worker;
use crate::inbox::{InboxPane, Reading, Stage};

/// A pane listing three threads, reading the second.
fn reading() -> InboxPane {
    let (tx, _) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.rows = ["a", "b", "c"]
        .iter()
        .map(|id| Thread {
            id: id.to_string(),
            from: format!("{id} <{id}@x.org>"),
            subject: format!("about {id}"),
            ..Thread::default()
        })
        .collect();
    p.sel = 1;
    let mut mail = Mail {
        from: "b <b@x.org>".into(),
        text: "Lunch?".into(),
        ..Mail::default()
    };
    mail.reply.subject = "about b".into();
    p.reading = Some(Reading {
        mails: Some(vec![mail]),
        ..Reading::new(p.rows[1].clone())
    });
    p
}

fn corners(p: &InboxPane, cols: u16, rows: u16, b: Rect) -> [char; 4] {
    let cells = p.cells(cols, rows);
    let at = |r: u16, c: u16| {
        cells
            .iter()
            .find(|x| x.row == r && x.col == c)
            .map_or(' ', |x| x.c)
    };
    let (r, c) = (b.row + b.h - 1, b.col + b.w - 1);
    [at(b.row, b.col), at(b.row, c), at(r, b.col), at(r, c)]
}

#[test]
fn the_list_the_mail_and_the_reply_are_each_a_card_and_none_overlap() {
    let _g = crate::app::motion_test_guard();
    let mut p = reading();
    let k = p.cards(160, 40);
    let (list, mail) = (k.list.expect("beside it"), k.mail.expect("the mail"));
    assert!(list.col + list.w < mail.col, "air between them");
    assert_eq!(
        corners(&p, 160, 40, mail),
        ['\u{256d}', '\u{256e}', '\u{2570}', '\u{256f}']
    );
    assert_eq!(
        corners(&p, 160, 40, list),
        ['\u{256d}', '\u{256e}', '\u{2570}', '\u{256f}']
    );
    p.press(&winit::keyboard::Key::Character("r".into()), 160, 40);
    let k = p.cards(160, 40);
    let (mail, reply) = (k.mail.unwrap(), k.reply.expect("the reply box"));
    assert_eq!(mail.row + mail.h, reply.row, "the box right under the mail");
    assert_eq!(
        (mail.col, mail.w),
        (reply.col, reply.w),
        "as wide as the mail"
    );
    assert_eq!(corners(&p, 160, 40, reply)[3], '\u{256f}');
    p.reading = None;
    p.compose = None;
    let k = p.cards(80, 30);
    assert!(k.mail.is_none() && k.list.is_some(), "the list alone");
}

#[test]
fn the_mail_card_is_always_taller_than_the_reply_box() {
    let _g = crate::app::motion_test_guard();
    for rows in [14u16, 20, 30, 50] {
        let mut p = reading();
        p.press(&winit::keyboard::Key::Character("r".into()), 90, rows);
        p.type_in(&"a long reply that keeps going on and on ".repeat(30));
        let k = p.cards(90, rows);
        let (mail, reply) = (k.mail.unwrap(), k.reply.unwrap());
        assert!(
            mail.h > reply.h,
            "{rows} rows: mail {} box {}",
            mail.h,
            reply.h
        );
    }
}

#[test]
fn the_subject_names_the_mail_card_and_its_counter_steps_through_the_list() {
    let _g = crate::app::motion_test_guard();
    let p = reading();
    let mail = p.cards(90, 30).mail.unwrap();
    let cells = p.cells(90, 30);
    let edge: String = {
        let mut on: Vec<_> = cells.iter().filter(|c| c.row == mail.row).collect();
        on.sort_by_key(|c| c.col);
        on.iter().map(|c| c.c).collect()
    };
    assert!(
        edge.contains("about b") && edge.contains("2 of 3"),
        "{edge}"
    );
    let next = cells
        .iter()
        .find(|c| c.row == mail.row && c.c == '\u{203a}')
        .expect("the arrow");
    assert_eq!(p.counter_at(90, 30, mail.row, next.col), Some(true));
    let prev = cells
        .iter()
        .find(|c| c.row == mail.row && c.c == '\u{2039}')
        .unwrap();
    assert_eq!(p.counter_at(90, 30, mail.row, prev.col), Some(false));
    assert_eq!(p.counter_at(90, 30, mail.row + 1, next.col), None);
}

#[test]
fn a_reply_being_written_keeps_its_mail_under_it() {
    let _g = crate::app::motion_test_guard();
    let mut p = reading();
    p.press(&winit::keyboard::Key::Character("r".into()), 160, 40);
    p.type_in("Friday then.");
    let list = p.cards(160, 40).list.unwrap().inner();
    let shown = p.shown(list.h);
    let (r, _) = shown.threads().find(|&(_, i)| i == 0).expect("thread a");
    p.click(160, 40, list.row + r, list.col + 4);
    let open = |p: &InboxPane| p.reading.as_ref().unwrap().thread.id.clone();
    assert_eq!(open(&p), "b", "a click on the list does not swap the mail");
    assert_eq!(p.note, crate::inbox::advance::HELD);
    let mail = p.cards(160, 40).mail.unwrap();
    let next = p
        .cells(160, 40)
        .into_iter()
        .find(|c| c.row == mail.row && c.c == '\u{203a}')
        .unwrap();
    p.click(160, 40, mail.row, next.col);
    assert_eq!(open(&p), "b", "nor does the counter");
    assert_eq!(p.compose.as_ref().unwrap().body.text, "Friday then.");
    p.compose = None;
    p.click(160, 40, mail.row, next.col);
    assert_eq!(open(&p), "c", "with no reply open, it steps on");
}

#[test]
fn the_setup_page_is_a_card_and_says_what_the_pane_does() {
    let p = InboxPane::with(Stage::Setup { found: None });
    let cells = p.cells(80, 24);
    let text: String = cells.iter().map(|c| c.c).collect();
    assert!(text.contains("set up Gmail"), "its card's name");
    assert!(cells
        .iter()
        .any(|c| (c.row, c.col, c.c) == (0, 1, '\u{256d}')));
    assert!(text.contains("read, file and reply"), "{text}");
    assert!(
        !text.contains("read-only"),
        "it has not been read-only since v0.27.30"
    );
}

#[test]
fn the_card_the_keys_go_to_has_the_stronger_edge() {
    let _g = crate::app::theme_test_guard();
    let mut p = reading();
    let corner = |p: &InboxPane, b: Rect| {
        p.cells(160, 40)
            .into_iter()
            .find(|c| (c.row, c.col) == (b.row, b.col))
            .map(|c| c.fg)
    };
    let (keyed, rest) = (super::keyed_edge(), crew_theme::theme().border_normal);
    let k = p.cards(160, 40);
    assert_eq!(
        corner(&p, k.mail.unwrap()),
        Some(keyed),
        "reading: the mail"
    );
    assert_eq!(corner(&p, k.list.unwrap()), Some(rest));
    p.press(&winit::keyboard::Key::Character("r".into()), 160, 40);
    let k = p.cards(160, 40);
    assert_eq!(
        corner(&p, k.reply.unwrap()),
        Some(keyed),
        "writing: the box"
    );
    assert_eq!(corner(&p, k.mail.unwrap()), Some(rest));
    assert_ne!(
        keyed,
        crew_theme::theme().border_focused,
        "never the pane's own stroke"
    );
}
