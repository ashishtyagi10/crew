use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

use serde_json::json;
use winit::keyboard::Key;

use super::{best, of, How};
use crate::inbox::api::Mail;
use crate::inbox::grant::{Client, Grant};
use crate::inbox::wire::Thread;
use crate::inbox::worker::{Ask, Worker};
use crate::inbox::{InboxPane, Reading, Stage};

const LIST: &str =
    "<mailto:leave@list.example.com?subject=Unsubscribe%20me>, <https://example.com/u?id=7>";

#[test]
fn the_best_way_out_is_one_click_then_a_mail_and_only_a_page_without_a_signature() {
    let one = best(LIST, true, true);
    assert_eq!(
        one,
        Some(How::OneClick("https://example.com/u?id=7".into()))
    );
    let mail = best(LIST, false, true);
    assert_eq!(
        mail,
        Some(How::Mail {
            to: "leave@list.example.com".into(),
            subject: "Unsubscribe me".into(),
            body: "unsubscribe".into(),
        })
    );
    assert_eq!(
        best(LIST, true, false),
        Some(How::Web("https://example.com/u?id=7".into())),
        "unsigned mail is never acted on, only opened"
    );
    assert_eq!(best("<mailto:a@b.co>", true, false), None);
    for host in [
        "http://example.com/u",
        "https://10.0.0.1/u",
        "https://localhost/u",
    ] {
        let got = best(&format!("<{host}>"), true, true);
        assert!(!matches!(got, Some(How::OneClick(_))), "{host}: {got:?}");
    }
    assert_eq!(best("", true, true), None);
}

#[test]
fn the_headers_say_the_way_and_whether_gmail_found_them_signed() {
    let msg = |auth: &str| {
        json!({"payload": {"headers": [
            {"name": "List-Unsubscribe", "value": LIST},
            {"name": "list-unsubscribe-post", "value": "List-Unsubscribe=One-Click"},
            {"name": "Authentication-Results", "value": auth},
        ]}})
    };
    let signed = of(&msg(
        "mx.google.com; dkim=pass header.i=@example.com; spf=pass",
    ));
    assert!(matches!(signed, Some(How::OneClick(_))), "{signed:?}");
    let forged = of(&msg("mx.google.com; dkim=fail; spf=softfail"));
    assert!(matches!(forged, Some(How::Web(_))), "{forged:?}");
    assert_eq!(of(&json!({"payload": {"headers": []}})), None);
}

fn reading(how: Option<How>) -> (InboxPane, mpsc::Receiver<Ask>) {
    let (tx, asks) = mpsc::channel();
    let (_, rx) = mpsc::channel();
    let mut p = InboxPane::with(Stage::Ready(Worker { tx, rx }));
    p.rows = vec![Thread::default()];
    let mail = Mail {
        from: "Acme News <news@acme.example>".into(),
        unsub: how,
        ..Mail::default()
    };
    p.reading = Some(Reading {
        mails: Some(vec![mail]),
        ..Reading::new(Thread::default())
    });
    (p, asks)
}

fn press(p: &mut InboxPane, k: &str) {
    p.press(&Key::Character(k.into()), 80, 30);
}

#[test]
fn u_says_what_it_would_do_and_a_second_u_does_it() {
    let (mut p, asks) = reading(Some(How::OneClick("https://acme.example/u".into())));
    press(&mut p, "u");
    assert!(
        p.note.starts_with("u again unsubscribes from Acme News"),
        "{}",
        p.note
    );
    assert!(asks.try_recv().is_err(), "nothing yet");
    press(&mut p, "j");
    press(&mut p, "u");
    assert!(asks.try_recv().is_err(), "another key between asks again");
    press(&mut p, "u");
    let got = asks.try_recv().expect("the second u acts");
    assert!(matches!(got, Ask::Unsubscribe { ref who, .. } if who == "Acme News"));
    assert!(!p.busy, "the list stays live");
    let (mut none, _) = reading(None);
    press(&mut none, "u");
    assert_eq!(none.note, "this mail gives no way to unsubscribe");
}

/// One request to a local stub, answered `status`: what was asked.
fn stub(status: u16) -> (String, mpsc::Receiver<String>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for mut s in l.incoming().flatten() {
            let mut buf = vec![0u8; 8192];
            let mut got = String::new();
            while !(got.contains("\r\n\r\n") && got.contains("One-Click") || got.contains("raw")) {
                match s.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => got.push_str(&String::from_utf8_lossy(&buf[..n])),
                }
                if got.starts_with("POST /token") && got.contains("\r\n\r\n") {
                    break;
                }
            }
            let token = got.starts_with("POST /token");
            let body = if token {
                r#"{"access_token":"a","expires_in":3600}"#
            } else {
                "{}"
            };
            let code = if token { 200 } else { status };
            let reply = format!(
                "HTTP/1.1 {code} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = s.write_all(reply.as_bytes());
            let _ = tx.send(got);
        }
    });
    (base, rx)
}

fn gmail(base: &str) -> crate::inbox::api::Gmail {
    let grant = Grant {
        client: Client {
            id: "c".into(),
            secret: "s".into(),
        },
        refresh_token: "1//r".into(),
        email: String::new(),
    };
    crate::inbox::api::Gmail::at(grant, &format!("{base}/token"), &format!("{base}/gmail")).unwrap()
}

#[test]
fn one_click_is_one_form_post_and_a_mail_is_sent_through_gmail() {
    let (base, seen) = stub(200);
    let mut g = gmail(&base);
    g.unsubscribe(&How::OneClick(format!("{base}/u?id=7")))
        .unwrap();
    let post = seen.recv().unwrap();
    assert!(post.starts_with("POST /u?id=7 "), "{post}");
    assert!(post.ends_with("List-Unsubscribe=One-Click"), "{post}");
    let how = How::Mail {
        to: "leave@list.example.com".into(),
        subject: "unsubscribe".into(),
        body: "unsubscribe".into(),
    };
    g.unsubscribe(&how).unwrap();
    let sent: Vec<String> = seen.try_iter().collect();
    assert!(
        sent.iter()
            .any(|r| r.starts_with("POST /gmail/messages/send")),
        "{sent:?}"
    );
    let (refused, _) = stub(404);
    let err = gmail(&refused).unsubscribe(&How::OneClick(format!("{refused}/u")));
    assert!(err.unwrap_err().contains("refused the unsubscribe"));
}
