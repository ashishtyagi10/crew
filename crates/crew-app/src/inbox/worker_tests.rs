use super::{Ask, Got, State};
use crate::inbox::api::Gmail;
use crate::inbox::forwardmail::tests::serve;
use crate::inbox::grant::{Client, Grant};

#[test]
fn a_filing_that_went_brings_the_unread_count_with_it() {
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
    let got = State::default().answer(
        &mut g,
        Ask::Relabel {
            id: "t1".into(),
            add: vec![],
            remove: vec!["UNREAD"],
        },
    );
    assert!(
        got.iter().any(|g| matches!(g, Got::Unread(_))),
        "the pane's count, at once"
    );
    let asked: Vec<String> = log
        .lock()
        .unwrap()
        .iter()
        .map(|r| r.lines().next().unwrap_or("").to_string())
        .collect();
    assert!(
        asked
            .iter()
            .any(|l| l.starts_with("GET /gmail/labels/INBOX")),
        "{asked:#?}"
    );
}

/// `j` at the end of the list goes on from the deepest page held: the check
/// that lists the first page again keeps the cursor of a list paged past it,
/// as the pane keeps those rows (else the next `j` fetched page two again).
/// A list asked afresh starts over, and Gmail's end ends it either way.
#[test]
fn a_fresh_first_page_keeps_the_deeper_cursor() {
    let mut s = State::default();
    s.went(false, Some("p2".into()));
    s.went(true, Some("p3".into()));
    s.went(false, Some("p2 again".into()));
    assert_eq!(s.next.as_deref(), Some("p3"));
    s.went(false, None);
    assert_eq!(s.next, None, "the first page is the whole list now");
    s.went(false, Some("p2".into()));
    s.went(true, None);
    s.went(false, Some("p2 again".into()));
    assert_eq!(s.next, None, "the pane holds the list to its end");
    s.pages = 0; // what `Ask::List` does: another search
    s.went(false, Some("q2".into()));
    assert_eq!(s.next.as_deref(), Some("q2"));
}

/// Offline when the worker started, the account was never told: `me` stayed
/// no one for the session, the list never said "me" and a reply-all wrote to
/// yourself too. It comes with the first list that reaches Gmail, once.
#[test]
fn the_account_comes_with_the_first_list_that_reaches_gmail() {
    let (base, _) = serve();
    let grant = Grant {
        client: Client {
            id: "c".into(),
            secret: "s".into(),
        },
        refresh_token: "1//r".into(),
        email: String::new(),
    };
    let mut g = Gmail::at(grant, &format!("{base}/token"), &format!("{base}/gmail")).unwrap();
    let mut s = State::default();
    let got = s.answer(&mut g, Ask::List(String::new()));
    assert!(
        matches!(got.first(), Some(Got::Account(e)) if e == "me@x.org"),
        "told, before the page"
    );
    let again = s.answer(&mut g, Ask::List(String::new()));
    assert!(!again.iter().any(|g| matches!(g, Got::Account(_))), "once");
}
