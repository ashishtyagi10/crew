use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::super::grant::{Client, Grant};
use super::Gmail;

type Route = Arc<dyn Fn(&str) -> (u16, String) + Send + Sync>;

/// A stub of Google's two hosts on one port: `/token` and `/gmail/...`. Every
/// request's path (with its query) is logged.
fn serve(route: Route) -> (String, Arc<Mutex<Vec<String>>>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    let log = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&log);
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            let (route, seen) = (Arc::clone(&route), Arc::clone(&seen));
            std::thread::spawn(move || answer(s, &route, &seen));
        }
    });
    (base, log)
}

fn answer(mut s: std::net::TcpStream, route: &Route, seen: &Mutex<Vec<String>>) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    while !String::from_utf8_lossy(&buf).contains("\r\n\r\n") {
        let n = s.read(&mut chunk).unwrap_or(0);
        if n == 0 {
            return;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let text = String::from_utf8_lossy(&buf).to_string();
    let path = text.split_whitespace().nth(1).unwrap_or("").to_string();
    seen.lock().unwrap().push(path.clone());
    let (status, body) = route(&path);
    let reply = format!(
        "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = s.write_all(reply.as_bytes());
}

fn grant() -> Grant {
    Grant {
        client: Client {
            id: "c".into(),
            secret: "s".into(),
        },
        refresh_token: "1//r".into(),
        email: String::new(),
    }
}

fn gmail(base: &str) -> Gmail {
    Gmail::at(grant(), &format!("{base}/token"), &format!("{base}/gmail")).unwrap()
}

const TOKEN: &str = r#"{"access_token":"ya29.x","expires_in":3599}"#;

fn thread_json(id: &str, subject: &str) -> String {
    format!(
        r#"{{"id":"{id}","messages":[{{"labelIds":["INBOX","UNREAD"],"internalDate":"5","snippet":"s","payload":{{"headers":[{{"name":"Subject","value":"{subject}"}},{{"name":"From","value":"Ann <a@x.org>"}}]}}}}]}}"#
    )
}

#[test]
fn a_page_is_the_list_then_every_thread_at_once_in_the_lists_order() {
    let route: Route = Arc::new(|p: &str| match p {
        "/token" => (200, TOKEN.into()),
        p if p.starts_with("/gmail/threads?") => (
            200,
            r#"{"threads":[{"id":"t1"},{"id":"t2"}],"nextPageToken":"n2"}"#.into(),
        ),
        p if p.starts_with("/gmail/threads/t1?") => {
            std::thread::sleep(std::time::Duration::from_millis(150));
            (200, thread_json("t1", "first"))
        }
        p if p.starts_with("/gmail/threads/t2?") => (200, thread_json("t2", "second")),
        _ => (404, "{}".into()),
    });
    let (base, log) = serve(route);
    let started = std::time::Instant::now();
    let (rows, next) = gmail(&base).page("", None).unwrap();
    let subjects: Vec<&str> = rows.iter().map(|t| t.subject.as_str()).collect();
    assert_eq!(
        subjects,
        ["first", "second"],
        "the list's order, not the arrival order"
    );
    assert_eq!(next.as_deref(), Some("n2"));
    assert!(rows.iter().all(|t| t.unread));
    let log = log.lock().unwrap().clone();
    assert!(
        log.iter()
            .any(|p| p.contains("q=in%3Ainbox") && p.contains("maxResults=25")),
        "{log:?}"
    );
    assert!(
        log.iter()
            .any(|p| p.contains("format=metadata&metadataHeaders=From")),
        "{log:?}"
    );
    assert!(started.elapsed() < std::time::Duration::from_millis(1500));
}

#[test]
fn a_401_renews_the_token_once_and_tries_again() {
    let calls = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&calls);
    let route: Route = Arc::new(move |p: &str| match p {
        "/token" => (200, TOKEN.into()),
        "/gmail/labels/INBOX" if c.fetch_add(1, Ordering::SeqCst) == 0 => (401, "{}".into()),
        "/gmail/labels/INBOX" => (200, r#"{"threadsUnread":7}"#.into()),
        _ => (404, "{}".into()),
    });
    let (base, log) = serve(route);
    assert_eq!(gmail(&base).unread().unwrap(), 7);
    let tokens = log
        .lock()
        .unwrap()
        .iter()
        .filter(|p| *p == "/token")
        .count();
    assert_eq!(tokens, 2, "renewed after the 401");
}

#[test]
fn gmails_own_error_is_said_and_an_opened_thread_reads_as_text() {
    let route: Route = Arc::new(|p: &str| match p {
        "/token" => (200, TOKEN.into()),
        p if p.starts_with("/gmail/threads/gone") => (
            404,
            r#"{"error":{"code":404,"message":"Requested entity was not found."}}"#.into(),
        ),
        p if p.starts_with("/gmail/threads/t1?format=full") => (
            200,
            format!(
                r#"{{"messages":[{{"payload":{{"mimeType":"text/plain","headers":[{{"name":"From","value":"Ann <a@x.org>"}},{{"name":"Date","value":"Mon"}}],"body":{{"data":"{}"}}}}}}]}}"#,
                "SGVsbG8"
            ),
        ),
        _ => (404, "{}".into()),
    });
    let (base, _) = serve(route);
    let mut g = gmail(&base);
    assert_eq!(
        g.open("gone").unwrap_err(),
        "Gmail 404: Requested entity was not found."
    );
    let mails = g.open("t1").unwrap();
    assert_eq!(
        (mails[0].from.as_str(), mails[0].text.as_str()),
        ("Ann <a@x.org>", "Hello")
    );
}

#[test]
fn delete_is_gmails_trash_and_undo_its_untrash() {
    let route: Route = Arc::new(|p: &str| match p {
        "/token" => (200, TOKEN.into()),
        _ => (200, "{}".into()),
    });
    let (base, log) = serve(route);
    let mut g = gmail(&base);
    g.trash("t1", false).unwrap();
    g.trash("t1", true).unwrap();
    let seen = log.lock().unwrap().clone();
    assert_eq!(
        seen,
        [
            "/token",
            "/gmail/threads/t1/trash",
            "/gmail/threads/t1/untrash"
        ]
    );
}
