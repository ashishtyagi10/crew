//! The minute's check lists the first page again when the mailbox moved
//! since the pane's first page saw it — not since any page, and not since a
//! page that failed.
use std::io::{Read, Write};
use std::net::TcpListener;

use super::super::{Ask, Got, State};
use crate::inbox::api::Gmail;
use crate::inbox::forwardmail::tests::serve;
use crate::inbox::grant::{Client, Grant};

fn gmail(base: &str) -> Gmail {
    let grant = Grant {
        client: Client {
            id: "c".into(),
            secret: "s".into(),
        },
        refresh_token: "1//r".into(),
        email: String::new(),
        scope: String::new(),
    };
    Gmail::at(grant, &format!("{base}/token"), &format!("{base}/gmail")).unwrap()
}

fn listed(got: &[Got]) -> bool {
    got.iter()
        .any(|g| matches!(g, Got::Page { more: false, .. }))
}

/// Mail landed on page one while `j` fetched page two: the check after must
/// still list page one, or that mail stays hidden.
#[test]
fn a_further_page_does_not_stand_for_the_first() {
    let (base, _) = serve();
    let mut g = gmail(&base);
    let mut s = State {
        history: "6".into(), // page one saw the mailbox at 6; it is at 7 now
        next: Some("p2".into()),
        told: true,
        ..State::default()
    };
    s.answer(&mut g, Ask::More);
    assert!(listed(&s.answer(&mut g, Ask::Check)), "page one again");
    assert!(
        matches!(s.answer(&mut g, Ask::Check).as_slice(), [Got::Checked]),
        "and then the mailbox is as page one saw it"
    );
}

/// Gmail answering 503 to the list, and everything else as it should.
fn serve_list_down() -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    std::thread::spawn(move || {
        for mut s in l.incoming().flatten() {
            let mut buf = [0u8; 65536];
            let n = s.read(&mut buf).unwrap_or(0);
            let head = String::from_utf8_lossy(&buf[..n]).to_string();
            let path = head.split_whitespace().nth(1).unwrap_or("").to_string();
            let (status, body) = match path.as_str() {
                "/token" => ("200 OK", r#"{"access_token":"ya29.x","expires_in":3599}"#),
                "/gmail/profile" => ("200 OK", r#"{"emailAddress":"me@x.org","historyId":"7"}"#),
                p if p.starts_with("/gmail/threads") => ("503 Unavailable", r#"{"error":{}}"#),
                _ => ("200 OK", r#"{"messagesUnread":0}"#),
            };
            let reply = format!(
                "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = s.write_all(reply.as_bytes());
        }
    });
    base
}

/// A first page that failed was never seen: the check after lists again
/// rather than calling an unlisted mailbox checked (the pane drew "inbox
/// zero · checked just now").
#[test]
fn a_failed_first_page_is_listed_again_by_the_check() {
    let mut g = gmail(&serve_list_down());
    // An earlier list saw the mailbox at 7, and it has not moved since.
    let mut s = State {
        history: "7".into(),
        told: true,
        ..State::default()
    };
    let first = s.answer(&mut g, Ask::List(String::new()));
    assert!(!listed(&first), "the list failed");
    let check = s.answer(&mut g, Ask::Check);
    assert!(
        !matches!(check.as_slice(), [Got::Checked]),
        "not checked: {}",
        check.len()
    );
}
