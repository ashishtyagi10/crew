use base64::Engine;

use super::super::api::Mail;
use super::{answer, answer_all, raw, Reply};

fn mail(from: &str, subject: &str, id: &str, reply_to: &str, refs: &str) -> Mail {
    Mail {
        from: from.into(),
        reply: Reply {
            subject: subject.into(),
            message_id: id.into(),
            reply_to: reply_to.into(),
            references: refs.into(),
            ..Reply::default()
        },
        ..Mail::default()
    }
}

fn decoded(r: &str) -> String {
    String::from_utf8(
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(r)
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn a_reply_answers_the_last_message_someone_else_sent_in_its_thread() {
    let mails = [
        mail("Ann <ann@x.org>", "Lunch?", "<1@x>", "", ""),
        mail("Me <me@gmail.com>", "Re: Lunch?", "<2@g>", "", "<1@x>"),
    ];
    let a = answer("t1", &mails, "me@gmail.com").unwrap();
    assert_eq!(
        (a.to.as_str(), a.subject.as_str()),
        ("Ann <ann@x.org>", "Re: Lunch?")
    );
    assert_eq!(
        (a.in_reply_to.as_str(), a.references.as_str()),
        ("<1@x>", "<1@x>")
    );
    let b = answer(
        "t1",
        &[mail(
            "Bo <bo@y.org>",
            "Re: plan",
            "<9@y>",
            "list@y.org",
            "<7@y> <8@y>",
        )],
        "me@gmail.com",
    )
    .unwrap();
    assert_eq!(
        (b.to.as_str(), b.subject.as_str()),
        ("list@y.org", "Re: plan"),
        "Reply-To wins, Re: once"
    );
    assert_eq!(b.references, "<7@y> <8@y> <9@y>");
    assert!(answer("t1", &[], "me@gmail.com").is_none());
}

#[test]
fn the_draft_carries_crews_headers_and_the_body_cannot_add_one() {
    let a = answer(
        "t1",
        &[mail("Ann <ann@x.org>", "Café plans", "<1@x>", "", "")],
        "",
    )
    .unwrap();
    let m = decoded(&raw(&a, "Thursday works.\nBcc: evil@x.org"));
    let (head, body) = m.split_once("\r\n\r\n").unwrap();
    assert!(
        head.starts_with("To: Ann <ann@x.org>\r\nSubject: =?UTF-8?B?"),
        "{head}"
    );
    assert!(
        head.contains("In-Reply-To: <1@x>\r\nReferences: <1@x>\r\n"),
        "{head}"
    );
    assert!(
        head.contains("Content-Type: text/plain; charset=UTF-8"),
        "{head}"
    );
    assert!(!head.contains("Bcc"), "the body stays the body: {head}");
    assert!(
        head.ends_with("Content-Transfer-Encoding: base64"),
        "{head}"
    );
    assert_eq!(words(body), "Thursday works.\r\nBcc: evil@x.org");
}

/// A base64 body, in its lines of 76, as its words.
fn words(body: &str) -> String {
    let b: String = body.split_whitespace().collect();
    let bytes = base64::engine::general_purpose::STANDARD.decode(b).unwrap();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn a_long_paragraph_goes_in_lines_a_mail_may_carry() {
    let a = answer(
        "t1",
        &[mail("Ann <ann@x.org>", "plans", "<1@x>", "", "")],
        "",
    )
    .unwrap();
    let long = "word ".repeat(400);
    let m = decoded(&raw(&a, &long));
    assert!(
        m.split("\r\n").all(|l| l.len() <= 998),
        "RFC 5322's line limit"
    );
    assert_eq!(words(m.split_once("\r\n\r\n").unwrap().1), long);
}

#[test]
fn reply_all_answers_everyone_but_me_and_a_cc_line_goes_out() {
    let mut last = mail("Ann Lee <ann@x.org>", "plans", "<1@x>", "", "");
    last.reply.to = "Me <ME@gmail.com>, \"Ray, Bo\" <bo@y.org>".into();
    last.reply.cc = "cy@z.org, ann@x.org, me@gmail.com".into();
    let a = answer_all("t1", &[last], "me@gmail.com").unwrap();
    assert_eq!(a.to, "Ann Lee <ann@x.org>, \"Ray, Bo\" <bo@y.org>");
    assert_eq!(a.cc, "cy@z.org", "once each, and never me");
    let head = decoded(&raw(&a, "ok"));
    assert!(
        head.contains("\r\nCc: cy@z.org\r\nSubject: Re: plans\r\n"),
        "{head}"
    );
    let plain = answer(
        "t1",
        &[mail("Ann <ann@x.org>", "plans", "<1@x>", "", "")],
        "",
    )
    .unwrap();
    assert!(
        !decoded(&raw(&plain, "ok")).contains("Cc:"),
        "no cc, no line"
    );
}

#[test]
fn a_header_value_with_a_line_break_cannot_start_another_header() {
    let mut a = answer(
        "t1",
        &[mail("Ann <ann@x.org>", "plans", "<1@x>", "", "")],
        "",
    )
    .unwrap();
    a.subject = "Re: plans\r\nBcc: evil@x.org".into();
    let m = decoded(&raw(&a, "ok"));
    let head = m.split_once("\r\n\r\n").unwrap().0;
    assert!(head.lines().all(|l| !l.starts_with("Bcc")), "{head}");
}
