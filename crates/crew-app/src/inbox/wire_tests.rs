use serde_json::json;

fn msg(from: &str, subject: &str, at: &str, labels: &[&str], snippet: &str) -> serde_json::Value {
    json!({
        "id": format!("m{at}"),
        "labelIds": labels,
        "snippet": snippet,
        "internalDate": at,
        "payload": {"headers": [
            {"name": "From", "value": from},
            {"name": "subject", "value": subject},
            {"name": "Date", "value": "Mon, 20 Oct 2026 09:00:00 -0400"}
        ]}
    })
}

#[test]
fn a_thread_reads_as_its_newest_sender_and_its_first_subject() {
    let v = json!({"id": "t1", "messages": [
        msg("Ann Lee <ann@x.org>", "Lunch?", "1000", &["INBOX"], "first"),
        msg("Bo <bo@y.org>", "Re: Lunch?", "3000", &["INBOX", "UNREAD"], "Sure &amp; see you at 1 &#39;ish"),
        msg("Cy <cy@z.org>", "Re: Lunch?", "2000", &["INBOX"], "middle"),
    ]});
    let t = super::thread(&v);
    assert_eq!(t.id, "t1");
    assert_eq!(t.from, "Bo <bo@y.org>");
    assert_eq!(t.subject, "Lunch?");
    assert_eq!(t.snippet, "Sure & see you at 1 'ish");
    assert!(t.unread);
    assert_eq!((t.at_ms, t.count), (3000, 3));
}

#[test]
fn a_read_thread_and_missing_fields_are_rows_not_errors() {
    let t = super::thread(&json!({"id": "t2", "messages": [{"labelIds": ["INBOX"]}]}));
    assert_eq!(
        (t.id.as_str(), t.subject.as_str(), t.unread, t.count),
        ("t2", "", false, 1)
    );
    assert_eq!(super::thread(&json!({})), super::Thread::default());
}

#[test]
fn the_list_gives_ids_in_order_and_the_next_page() {
    let v = json!({"threads": [{"id": "a"}, {"id": "b"}], "nextPageToken": "p2"});
    assert_eq!(
        super::thread_ids(&v),
        (vec!["a".into(), "b".into()], Some("p2".into()))
    );
    assert_eq!(
        super::thread_ids(&json!({"resultSizeEstimate": 0})),
        (vec![], None)
    );
}

#[test]
fn the_inbox_label_counts_unread_threads() {
    assert_eq!(
        super::unread_threads(&json!({"id": "INBOX", "threadsUnread": 12})),
        12
    );
    assert_eq!(super::unread_threads(&json!({})), 0);
}

#[test]
fn a_sender_is_named_by_their_display_name() {
    assert_eq!(super::sender_name("Ann Lee <ann@x.org>"), "Ann Lee");
    assert_eq!(super::sender_name("\"Lee, Ann\" <ann@x.org>"), "Lee, Ann");
    assert_eq!(super::sender_name("<ann@x.org>"), "ann@x.org");
    assert_eq!(super::sender_name("ann@x.org"), "ann@x.org");
}

#[test]
fn senders_are_every_written_message_oldest_first_and_never_a_draft() {
    let v = json!({"id": "t3", "messages": [
        msg("Bo <bo@y.org>", "Re: x", "3000", &["INBOX"], ""),
        msg("Ann <ann@x.org>", "x", "1000", &["INBOX"], ""),
        msg("Me <me@x.org>", "Re: x", "2000", &["SENT"], ""),
        msg("Me <me@x.org>", "Re: x", "4000", &["DRAFT"], ""),
    ]});
    let got: Vec<(String, bool)> = super::thread(&v)
        .senders
        .into_iter()
        .map(|s| (s.from, s.sent))
        .collect();
    assert_eq!(
        got,
        [
            ("Ann <ann@x.org>".to_string(), false),
            ("Me <me@x.org>".to_string(), true),
            ("Bo <bo@y.org>".to_string(), false),
        ]
    );
}

#[test]
fn a_draft_and_an_attachment_come_with_the_metadata() {
    let mut first = msg("Ann <ann@x.org>", "x", "1000", &["INBOX", "Label_7"], "");
    first["payload"]["headers"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name": "Content-Type", "value": "Multipart/Mixed; boundary=\"b\""}));
    let v = json!({"id": "t4", "messages": [
        first,
        msg("Me <me@x.org>", "Re: x", "2000", &["SENT", "IMPORTANT", "INBOX"], ""),
        msg("Me <me@x.org>", "Re: x", "3000", &["DRAFT"], ""),
    ]});
    let t = super::thread(&v);
    assert!(t.draft, "a message still a draft");
    assert!(t.attached, "a multipart/mixed message");
    let plain = super::thread(&json!({"id": "t5", "messages": [
        msg("Ann <ann@x.org>", "x", "1000", &["INBOX"], ""),
    ]}));
    assert!(!plain.draft && !plain.attached);
}
