use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use base64::Engine;

use super::tree;
use crate::inbox::api::Gmail;
use crate::inbox::forward::tests::mail;
use crate::inbox::forward::Forwarded;
use crate::inbox::grant::{Client, Grant};

#[test]
fn the_forward_is_your_note_then_gmails_rule_and_the_mail_itself() {
    let f = Forwarded::of(&mail());
    let m = crate::inbox::mime::message(&[], &tree("FYI <3", &f, vec![vec![7], vec![8]], vec![]));
    let decoded: Vec<String> = m
        .split("\r\n\r\n")
        .skip(1)
        .filter_map(|b| {
            let b: String = b.lines().take_while(|l| !l.starts_with("--")).collect();
            let bytes = base64::engine::general_purpose::STANDARD.decode(b).ok()?;
            String::from_utf8(bytes).ok()
        })
        .collect();
    let plain = decoded
        .iter()
        .find(|d| d.starts_with("FYI <3"))
        .expect("plain words");
    assert!(plain.contains("---------- Forwarded message ---------\r\nFrom: Ann Lee <ann@x.org>\r\nDate: Tue, 21 Oct 2026\r\nSubject: Lunch\r\nTo: me@y.org\r\n"), "{plain}");
    assert!(
        plain.ends_with("**Hi** there\r\n"),
        "no plain part: the reader's words"
    );
    let html = decoded
        .iter()
        .find(|d| d.starts_with("<div>"))
        .expect("html");
    assert!(html.starts_with("<div>FYI &lt;3</div>"), "{html}");
    assert!(
        html.contains("From: Ann Lee &lt;ann@x.org&gt;<br>"),
        "{html}"
    );
    assert!(
        html.contains(
            "<br><style>.a{color:red}</style><p class=\"a\">Hi <img src=\"cid:logo@x\"></p></div>"
        ),
        "the body, its style kept: {html}"
    );
    assert!(!html.contains("<html") && !html.contains("<body"), "{html}");
    for want in [
        "multipart/mixed",
        "multipart/related",
        "multipart/alternative",
        "Content-ID: <logo@x>",
        "filename=\"menu.pdf\"",
    ] {
        assert!(m.contains(want), "{want}: {m}");
    }
}

/// A stub of Google: every request's first line, headers and body logged.
pub(crate) fn serve() -> (String, Arc<Mutex<Vec<String>>>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    let log = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&log);
    std::thread::spawn(move || {
        for mut s in l.incoming().flatten() {
            let mut buf = Vec::new();
            let mut chunk = [0u8; 65536];
            let whole = |b: &[u8]| {
                let t = String::from_utf8_lossy(b);
                let Some((head, body)) = t.split_once("\r\n\r\n") else {
                    return false;
                };
                let len = head.lines().find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|n| n.trim().parse::<usize>().unwrap_or(0))
                });
                body.len() >= len.unwrap_or(0)
            };
            while !whole(&buf) {
                match s.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => buf.extend_from_slice(&chunk[..n]),
                }
            }
            let text = String::from_utf8_lossy(&buf).to_string();
            let path = text.split_whitespace().nth(1).unwrap_or("").to_string();
            seen.lock().unwrap().push(text);
            let body = match path.as_str() {
                "/token" => r#"{"access_token":"ya29.x","expires_in":3599}"#.to_string(),
                "/gmail/profile" => r#"{"emailAddress":"me@x.org","historyId":"7"}"#.to_string(),
                p if p.contains("/attachments/a3") => r#"{"data":"JVBERg"}"#.to_string(),
                p if p.contains("/attachments/a1") => r#"{"data":"iVBO"}"#.to_string(),
                _ => r#"{"id":"sent"}"#.to_string(),
            };
            let reply = format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
            let _ = s.write_all(reply.as_bytes());
        }
    });
    (base, log)
}

#[test]
fn send_fetches_the_files_then_uploads_the_mail_as_itself() {
    let (base, log) = serve();
    let grant = Grant {
        client: Client {
            id: "c".into(),
            secret: "s".into(),
        },
        refresh_token: "1//r".into(),
        email: String::new(),
    };
    let mut g = Gmail::at(grant, &format!("{base}/token"), &format!("{base}/gmail")).unwrap();
    let f = Forwarded::of(&mail());
    let o = crate::inbox::outgoing::Outgoing {
        head: vec!["To: Dee <dee@x.org>".into(), "Subject: Fwd: Lunch".into()],
        thread_id: None,
        words: "FYI".into(),
        forward: Some(Box::new(f)),
        files: vec![],
        to: "Dee <dee@x.org>".into(),
    };
    crate::inbox::outgoing::send(&mut g, &o).expect("sent");
    let log = log.lock().unwrap().clone();
    let at = |p: &str| {
        log.iter()
            .position(|r| r.starts_with(&format!("GET {p}")))
            .unwrap_or(usize::MAX)
    };
    assert!(
        at("/gmail/messages/m1/attachments/a1") < usize::MAX
            && at("/gmail/messages/m1/attachments/a3") < usize::MAX,
        "{log:#?}"
    );
    let up = log
        .iter()
        .find(|r| r.starts_with("POST /upload/gmail/messages/send?uploadType=media "))
        .expect("the upload");
    assert!(
        up.to_ascii_lowercase()
            .contains("content-type: message/rfc822"),
        "{up}"
    );
    assert!(
        up.contains("\r\n\r\nTo: Dee <dee@x.org>\r\nSubject: Fwd: Lunch\r\nMIME-Version: 1.0\r\n"),
        "{up}"
    );
    assert!(
        up.contains("filename=\"menu.pdf\"") && up.contains("Content-ID: <logo@x>"),
        "{up}"
    );
}
