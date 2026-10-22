use crate::chatbody::CardLine;
use crate::inbox::api::Mail;
use crate::inbox::ics::tests::{cal, GOOGLE};
use crate::inbox::{wire::Thread, Reading};

/// The open thread's lines for one mail carrying `ics`.
fn lines(ics: Option<String>) -> Vec<CardLine> {
    let m = Mail {
        id: "m1".into(),
        from: "Ann Lee <ann@x.org>".into(),
        text: "See you there.".into(),
        calendar: ics,
        ..Default::default()
    };
    let r = Reading {
        mails: Some(vec![m]),
        ..Reading::new(Thread::default())
    };
    r.laid(60).lines.clone()
}

fn text(l: &CardLine) -> String {
    l.iter().map(|c| c.c).collect()
}

fn card(lines: &[CardLine]) -> Vec<String> {
    lines
        .iter()
        .map(text)
        .filter(|l| l.starts_with(" \u{258e} "))
        .collect()
}

#[test]
fn an_invite_is_a_card_under_its_sender_and_over_its_words() {
    let all = lines(Some(cal("METHOD:REQUEST\r\n", GOOGLE)));
    let c = card(&all);
    assert!(c[0].ends_with("Q4 planning"), "{c:?}");
    assert!(c[1].contains("Oct 3") && c[1].contains('\u{2013}'), "{c:?}");
    assert!(c.iter().any(|l| l.ends_with("Room 4, east")), "{c:?}");
    assert!(
        c.iter().any(|l| l.ends_with("Ann Lee \u{b7} 2 invited")),
        "{c:?}"
    );
    let n = c.len();
    assert!(c[n - 2].ends_with("meet.google.com/abc-defg-hij"), "{c:?}");
    let keys = "c adds it to your calendar \u{b7} y answers it";
    assert!(c[n - 1].ends_with(keys), "{c:?}");
    let at = |needle: &str| all.iter().position(|l| text(l).contains(needle)).unwrap();
    assert!(at("Ann Lee") < at("Q4 planning") && at("Q4 planning") < at("See you there."));
    let title = all
        .iter()
        .find(|l| text(l).ends_with("Q4 planning"))
        .unwrap();
    assert!(title.last().unwrap().bold, "the title is bold");
}

#[test]
fn the_join_link_clicks_to_the_whole_address_and_shows_no_password() {
    let url = "https://us02web.zoom.us/j/8812345678?pwd=abc";
    let ev = format!("DTSTART:20261104T150000Z\r\nSUMMARY:Crit\r\nLOCATION:{url}\r\n");
    let all = lines(Some(cal("", &ev)));
    let row = all
        .iter()
        .find(|l| text(l).ends_with("zoom.us/j/8812345678"))
        .unwrap();
    let linked = row.iter().find(|c| c.c == 'z').and_then(|c| c.link.clone());
    assert_eq!(linked.as_deref(), Some(url));
    assert_eq!(
        card(&all).iter().filter(|l| l.contains("zoom")).count(),
        1,
        "a place that is the link says it once"
    );
}

#[test]
fn a_cancelled_invite_says_so_first_and_a_mail_with_none_has_no_card() {
    let ev = "DTSTART:20261102T190000Z\r\nSUMMARY:Hiring panel\r\n";
    let c = card(&lines(Some(cal("METHOD:CANCEL\r\n", ev))));
    assert!(c[0].ends_with("cancelled \u{b7} Hiring panel"), "{c:?}");
    assert!(!c.iter().any(|l| l.contains("c adds")), "nothing to add");
    assert!(card(&lines(None)).is_empty());
    assert!(card(&lines(Some("BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n".into()))).is_empty());
}
