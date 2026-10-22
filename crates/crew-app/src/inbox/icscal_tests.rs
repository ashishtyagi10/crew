use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use super::{add, answer, event, seen, SCOPE};
use crate::inbox::api::Gmail;
use crate::inbox::grant::{Client, Grant};
use crate::inbox::ics::parse;
use crate::inbox::ics::reply::Answer;
use crate::inbox::ics::tests::{cal, GOOGLE};
use crate::inbox::worker::Got;

const PEOPLE: &str = r#"[{"email":"bo@y.org","responseStatus":"accepted"},{"email":"me@x.org","self":true,"responseStatus":"needsAction"}]"#;

/// A stub of Google: what each request line gets, every request kept whole.
fn serve() -> (String, Arc<Mutex<Vec<String>>>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    let log = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&log);
    std::thread::spawn(move || {
        for mut s in l.incoming().flatten() {
            let mut buf = Vec::new();
            let mut chunk = [0u8; 65536];
            loop {
                let t = String::from_utf8_lossy(&buf).to_string();
                if let Some((head, body)) = t.split_once("\r\n\r\n") {
                    let len = head.lines().find_map(|l| {
                        let l = l.to_ascii_lowercase();
                        l.strip_prefix("content-length:")
                            .map(|n| n.trim().parse().unwrap_or(0))
                    });
                    if body.len() >= len.unwrap_or(0) {
                        break;
                    }
                }
                match s.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => buf.extend_from_slice(&chunk[..n]),
                }
            }
            let text = String::from_utf8_lossy(&buf).to_string();
            let line = text.lines().next().unwrap_or_default().to_string();
            seen.lock().unwrap().push(text);
            let body = match line.as_str() {
                l if l.contains("/token") => {
                    r#"{"access_token":"ya29.x","expires_in":3599}"#.to_string()
                }
                l if l.contains("iCalUID=") => {
                    format!(r#"{{"items":[{{"id":"ev1","attendees":{PEOPLE}}}]}}"#)
                }
                l if l.starts_with("GET") && l.contains("/events/ev1") => {
                    format!(r#"{{"id":"ev1","attendees":{PEOPLE}}}"#)
                }
                _ => r#"{"id":"ok"}"#.to_string(),
            };
            let reply = format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
            let _ = s.write_all(reply.as_bytes());
        }
    });
    (base, log)
}

fn gmail(base: &str, scope: &str) -> Gmail {
    let grant = Grant {
        client: Client {
            id: "c".into(),
            secret: "s".into(),
        },
        refresh_token: "1//r".into(),
        email: String::new(),
        scope: scope.into(),
    };
    Gmail::at(grant, &format!("{base}/token"), &format!("{base}/gmail")).unwrap()
}

fn ics() -> String {
    cal("METHOD:REQUEST\r\n", GOOGLE)
}

/// The requests other than the token's, as `METHOD path` and body.
fn asked(log: &Arc<Mutex<Vec<String>>>) -> Vec<(String, String)> {
    let all = log.lock().unwrap().clone();
    let split = |t: &String| {
        let (head, body) = t.split_once("\r\n\r\n").unwrap_or((t, ""));
        let line = head.lines().next().unwrap_or_default();
        let words: Vec<&str> = line.split_whitespace().take(2).collect();
        (words.join(" "), body.to_string())
    };
    all.iter()
        .map(split)
        .filter(|(l, _)| !l.contains("/token"))
        .collect()
}

#[test]
fn without_the_calendar_scope_google_calendar_is_never_asked() {
    let (base, log) = serve();
    let mut g = gmail(&base, "https://www.googleapis.com/auth/gmail.modify");
    assert!(seen(&mut g, &ics()).is_empty());
    assert!(asked(&log).is_empty(), "{:?}", asked(&log));
}

#[test]
fn an_invite_on_google_calendar_says_your_answer_there() {
    let (base, log) = serve();
    let mut g = gmail(&base, &format!("gmail {SCOPE}"));
    let got = seen(&mut g, &ics());
    assert!(
        matches!(&got[..], [Got::OnCalendar(uid, Some(s))] if uid == "abc@google.com" && s == "needsAction")
    );
    assert_eq!(
        asked(&log)[0].0,
        "GET /gmail/calendar/events?iCalUID=abc%40google.com"
    );
}

#[test]
fn an_answer_on_google_calendar_sets_yours_and_google_tells_the_organizer() {
    let (base, log) = serve();
    let mut g = gmail(&base, SCOPE);
    let got = answer(&mut g, &ics(), Answer::Yes, "me@x.org");
    assert!(
        matches!(&got[..], [Got::Answered(_, s, line)] if s == "accepted" && line.contains("Ann Lee was told"))
    );
    let reqs = asked(&log);
    let (line, body) = reqs
        .iter()
        .find(|(l, _)| l.starts_with("PATCH"))
        .expect("a patch");
    assert_eq!(line, "PATCH /gmail/calendar/events/ev1?sendUpdates=all");
    let v: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(v["attendees"][1]["responseStatus"], "accepted", "yours");
    assert_eq!(
        v["attendees"][0]["responseStatus"], "accepted",
        "Bo's as Bo left it"
    );
}

#[test]
fn without_the_scope_an_answer_goes_to_the_organizer_as_a_mail() {
    let (base, log) = serve();
    let mut g = gmail(&base, "");
    let got = answer(&mut g, &ics(), Answer::No, "me@x.org");
    assert!(matches!(&got[..], [Got::Answered(_, s, _)] if s == "declined"));
    let reqs = asked(&log);
    assert_eq!(reqs.len(), 1, "{reqs:?}");
    assert_eq!(reqs[0].0, "POST /gmail/messages/send");
    let raw = serde_json::from_str::<serde_json::Value>(&reqs[0].1).unwrap()["raw"]
        .as_str()
        .unwrap()
        .to_string();
    use base64::Engine;
    let m = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(raw)
        .unwrap();
    assert!(String::from_utf8_lossy(&m).contains("Subject: Declined: Q4 planning"));
}

#[test]
fn adding_imports_the_invite_with_its_uid_its_zone_and_its_people() {
    let (base, log) = serve();
    let mut g = gmail(&base, SCOPE);
    assert!(matches!(&add(&mut g, &ics())[..], [Got::Calendar(line)] if line.contains("added")));
    let reqs = asked(&log);
    assert_eq!(reqs[0].0, "POST /gmail/calendar/events/import");
    let v: serde_json::Value = serde_json::from_str(&reqs[0].1).unwrap();
    assert_eq!(v["iCalUID"], "abc@google.com");
    assert_eq!(v["start"]["dateTime"], "2026-10-30T14:00:00Z");
    assert_eq!(v["start"]["timeZone"], "America/New_York");
    assert_eq!(v["organizer"]["email"], "ann@x.org");
    assert_eq!(v["attendees"][0]["responseStatus"], "accepted");
}

#[test]
fn a_whole_day_is_a_date_and_a_series_keeps_its_rule() {
    let ev = "UID:d\r\nDTSTART;VALUE=DATE:20261126\r\nDTEND;VALUE=DATE:20261127\r\nRRULE:FREQ=YEARLY\r\n";
    let v = event(&parse(&cal("", ev)).unwrap());
    assert_eq!(v["start"], serde_json::json!({ "date": "2026-11-26" }));
    assert_eq!(v["recurrence"], serde_json::json!(["RRULE:FREQ=YEARLY"]));
}

#[test]
fn inbox_calendar_asks_for_google_calendars_events_beside_gmails_and_needs_a_sign_in() {
    let u = crate::inbox::signin::authorize_url("c", "http://127.0.0.1:1", "C", "S", true);
    let scope = u
        .split('&')
        .find_map(|kv| kv.strip_prefix("scope="))
        .unwrap();
    let want = format!("{} {SCOPE}", crate::inbox::signin::SCOPE);
    assert_eq!(crate::inbox::query::dec(scope), want);
    let mut app = crate::app::CrewApp::default();
    app.inbox_command("calendar");
    let said = app.status.map(|(s, _)| s).unwrap_or_default();
    assert!(
        app.panes.is_empty() && said.contains("sign in first"),
        "{said}"
    );
}
