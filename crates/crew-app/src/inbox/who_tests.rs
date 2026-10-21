use super::names;
use crate::inbox::wire::{Sender, Thread};

fn thread(senders: &[(&str, bool)]) -> Thread {
    Thread {
        from: senders.last().map_or("", |s| s.0).into(),
        senders: senders
            .iter()
            .map(|(f, sent)| Sender {
                from: f.to_string(),
                sent: *sent,
            })
            .collect(),
        ..Thread::default()
    }
}

#[test]
fn one_sender_is_named_in_full_and_several_by_first_names_newest_last() {
    let one = thread(&[("Ann Lee <ann@x.org>", false)]);
    assert_eq!(names(&one, "", 24, true), "Ann Lee");
    let many = thread(&[
        ("Ann Lee <ann@x.org>", false),
        ("Bo Park <bo@y.org>", false),
        ("Ann Lee <ann@x.org>", false),
        ("Me <me@x.org>", true),
    ]);
    assert_eq!(names(&many, "", 40, true), "Bo Park, Ann Lee, me");
    assert_eq!(names(&many, "", 14, true), "Bo, Ann, me", "first names");
    assert_eq!(names(&many, "", 9, true), "Bo \u{2026} me");
    assert_eq!(names(&many, "", 40, false), "me", "the newest alone");
}

#[test]
fn the_account_is_me_by_its_address_too_and_a_bare_address_goes_by_its_name() {
    let t = thread(&[("noreply@github.com", false), ("Ash <Ash@Mail.org>", false)]);
    assert_eq!(
        names(&t, "ash@mail.org", 40, true),
        "noreply@github.com, me"
    );
    assert_eq!(names(&t, "ash@mail.org", 12, true), "noreply, me");
    assert_eq!(names(&t, "", 4, false), "Ash", "the newest alone");
    assert_eq!(names(&t, "", 2, false), "A\u{2026}", "cut to the room");
    assert_eq!(names(&Thread::default(), "", 10, true), "");
}
